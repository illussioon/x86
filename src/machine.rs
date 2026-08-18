use crate::bootloader::{Bootloader, FetchOptions, Resource, load_resource};
use crate::error::{Result, X86Error};
use crate::image::{Image, ImageKind};
use crate::state::SavedState;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsoleMode {
    Text,
    Headless,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleConfig {
    pub mode: ConsoleMode,
    pub echo_input: bool,
    pub width: u16,
    pub height: u16,
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            mode: ConsoleMode::Text,
            echo_input: true,
            width: 80,
            height: 25,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineConfig {
    pub ram_bytes: u64,
    pub vga_memory_bytes: u64,
    pub cpu_hz: u64,
    pub command_line: Option<String>,
    pub console: ConsoleConfig,
}

impl Default for MachineConfig {
    fn default() -> Self {
        Self {
            ram_bytes: 128 * 1024 * 1024,
            vga_memory_bytes: 8 * 1024 * 1024,
            cpu_hz: 1_000_000,
            command_line: None,
            console: ConsoleConfig::default(),
        }
    }
}

impl MachineConfig {
    pub fn with_ram_bytes(mut self, bytes: u64) -> Self {
        self.ram_bytes = bytes;
        self
    }

    pub fn with_vga_memory_bytes(mut self, bytes: u64) -> Self {
        self.vga_memory_bytes = bytes;
        self
    }

    pub fn with_cpu_hz(mut self, hz: u64) -> Self {
        self.cpu_hz = hz;
        self
    }

    pub fn with_command_line(mut self, command_line: impl Into<String>) -> Self {
        self.command_line = Some(command_line.into());
        self
    }

    pub fn with_console(mut self, console: ConsoleConfig) -> Self {
        self.console = console;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineStatus {
    Created,
    Ready,
    Running,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunOptions {
    pub max_steps: Option<u64>,
    pub quantum: Duration,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            max_steps: None,
            quantum: Duration::from_millis(10),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunReport {
    pub steps: u64,
    pub halted: bool,
}

pub trait ExecutionBackend: Send {
    fn reset(&mut self, config: &MachineConfig) -> Result<()>;

    /// Restore an attached v86 saved state after reset. Backends that do not
    /// support saved states may keep the default no-op implementation.
    fn restore_state(&mut self, _state: &SavedState) -> Result<()> {
        Ok(())
    }

    fn step(&mut self) -> Result<bool>;
    fn read_memory(&self, address: u64, buffer: &mut [u8]) -> Result<()>;
    fn write_memory(&mut self, address: u64, data: &[u8]) -> Result<()>;
}

pub struct Machine {
    config: MachineConfig,
    status: MachineStatus,
    bios: Option<Image>,
    vga_bios: Option<Image>,
    disk: Option<Image>,
    cdrom: Option<Image>,
    bootloader: Option<Bootloader>,
    saved_state: Option<SavedState>,
    backend: Option<Box<dyn ExecutionBackend>>,
}

impl Machine {
    pub fn new(config: MachineConfig) -> Self {
        Self {
            config,
            status: MachineStatus::Created,
            bios: None,
            vga_bios: None,
            disk: None,
            cdrom: None,
            bootloader: None,
            saved_state: None,
            backend: None,
        }
    }

    pub fn config(&self) -> &MachineConfig {
        &self.config
    }

    pub fn status(&self) -> MachineStatus {
        self.status
    }

    pub fn config_mut(&mut self) -> &mut MachineConfig {
        &mut self.config
    }

    pub fn set_ram_bytes(&mut self, bytes: u64) {
        self.config.ram_bytes = bytes;
    }

    pub fn set_vga_memory_bytes(&mut self, bytes: u64) {
        self.config.vga_memory_bytes = bytes;
    }

    pub fn set_command_line(&mut self, command_line: impl Into<String>) {
        self.config.command_line = Some(command_line.into());
    }

    pub fn attach_backend(&mut self, backend: impl ExecutionBackend + 'static) {
        self.backend = Some(Box::new(backend));
    }

    pub fn set_bios(&mut self, image: Image) -> Result<()> {
        require_kind(&image, ImageKind::Bios)?;
        self.bios = Some(image);
        Ok(())
    }

    pub fn set_vga_bios(&mut self, image: Image) -> Result<()> {
        require_kind(&image, ImageKind::VgaBios)?;
        self.vga_bios = Some(image);
        Ok(())
    }

    pub fn set_disk(&mut self, image: Image) -> Result<()> {
        self.disk = Some(image);
        Ok(())
    }

    pub fn set_cdrom(&mut self, image: Image) -> Result<()> {
        self.cdrom = Some(image);
        Ok(())
    }

    pub fn set_bootloader(&mut self, bootloader: Bootloader) {
        self.bootloader = Some(bootloader);
    }

    pub fn load_bootloader(&mut self, source: Resource) -> Result<()> {
        self.bootloader = Some(Bootloader::load(source)?);
        Ok(())
    }

    pub fn load_image(&mut self, kind: ImageKind, source: Resource) -> Result<()> {
        let image = load_resource(&source, kind, &FetchOptions::default())?;
        match kind {
            ImageKind::Bios => self.set_bios(image),
            ImageKind::VgaBios => self.set_vga_bios(image),
            ImageKind::RawDisk => self.set_disk(image),
            ImageKind::Iso9660 => self.set_cdrom(image),
            _ => Err(X86Error::InvalidImage(format!(
                "image kind {:?} cannot be attached as a machine device",
                kind
            ))),
        }
    }

    pub fn set_saved_state(&mut self, state: SavedState) {
        if let Some(memory_bytes) = state.memory_bytes() {
            self.config.ram_bytes = memory_bytes;
        }
        self.saved_state = Some(state);
    }

    pub fn load_saved_state(&mut self, source: Resource) -> Result<()> {
        let state_bytes = match source {
            Resource::File(path) => {
                std::fs::read(&path).map_err(|source| X86Error::Io { path, source })?
            }
            Resource::Bytes { bytes, .. } => bytes,
            Resource::Url(url) => load_resource(
                &Resource::Url(url),
                ImageKind::SavedState,
                &FetchOptions::default(),
            )?
            .bytes()
            .to_vec(),
        };
        let state = SavedState::from_bytes(state_bytes)?;
        if let Some(memory_bytes) = state.memory_bytes() {
            self.config.ram_bytes = memory_bytes;
        }
        self.saved_state = Some(state);
        Ok(())
    }

    pub fn bios(&self) -> Option<&Image> {
        self.bios.as_ref()
    }

    pub fn vga_bios(&self) -> Option<&Image> {
        self.vga_bios.as_ref()
    }

    pub fn disk(&self) -> Option<&Image> {
        self.disk.as_ref()
    }

    pub fn cdrom(&self) -> Option<&Image> {
        self.cdrom.as_ref()
    }

    pub fn bootloader(&self) -> Option<&Bootloader> {
        self.bootloader.as_ref()
    }

    pub fn saved_state(&self) -> Option<&SavedState> {
        self.saved_state.as_ref()
    }

    pub fn prepare(&mut self) -> Result<()> {
        if self.backend.is_none() {
            return Err(X86Error::BackendUnavailable(
                "no ExecutionBackend attached; attach a native CPU/device backend before run"
                    .to_owned(),
            ));
        }
        let backend = self.backend.as_mut().unwrap();
        backend.reset(&self.config)?;
        if let Some(state) = self.saved_state.as_ref() {
            backend.restore_state(state)?;
        }
        self.status = MachineStatus::Ready;
        Ok(())
    }

    pub fn run(&mut self, options: RunOptions) -> Result<RunReport> {
        if self.status == MachineStatus::Created {
            self.prepare()?;
        }
        let backend = self.backend.as_mut().ok_or_else(|| {
            X86Error::BackendUnavailable("no ExecutionBackend attached".to_owned())
        })?;
        self.status = MachineStatus::Running;
        let mut steps = 0;
        loop {
            if options.max_steps.is_some_and(|max| steps >= max) {
                break;
            }
            if backend.step()? {
                self.status = MachineStatus::Stopped;
                return Ok(RunReport {
                    steps,
                    halted: true,
                });
            }
            steps += 1;
        }
        Ok(RunReport {
            steps,
            halted: false,
        })
    }

    pub fn stop(&mut self) {
        self.status = MachineStatus::Stopped;
    }
}

fn require_kind(image: &Image, expected: ImageKind) -> Result<()> {
    if image.kind() != expected {
        return Err(X86Error::InvalidImage(format!(
            "expected {:?}, got {:?}",
            expected,
            image.kind()
        )));
    }
    Ok(())
}
