use crate::error::{Result, X86Error};
use crate::machine::{ExecutionBackend, MachineConfig};
use crate::state::SavedState;
use native_v86_core::native_runtime::NativeCpu;
use std::path::{Path, PathBuf};

/// Native x86 interpreter backend backed by the Rust port of v86's CPU core.
///
/// The current adapter is deliberately single-machine: the v86 CPU core uses
/// process-global pointers, so two NativeBackend instances must not execute at
/// the same time in one process.
pub struct NativeBackend {
    cpu: Option<NativeCpu>,
    instructions_per_step: u32,
    ninep_root: Option<PathBuf>,
}

impl NativeBackend {
    pub fn new() -> Self {
        Self {
            cpu: None,
            instructions_per_step: 10_000,
            ninep_root: None,
        }
    }

    pub fn with_instructions_per_step(mut self, value: u32) -> Self {
        self.instructions_per_step = value.max(1);
        self
    }

    pub fn cpu(&self) -> Option<&NativeCpu> {
        self.cpu.as_ref()
    }

    pub fn cpu_mut(&mut self) -> Option<&mut NativeCpu> {
        self.cpu.as_mut()
    }

    pub fn with_9p_root(mut self, path: impl AsRef<Path>) -> Self {
        self.ninep_root = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn set_9p_root(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(cpu) = self.cpu.as_mut() {
            cpu.set_9p_root(path)
                .map_err(X86Error::BackendUnavailable)?;
        }
        self.ninep_root = Some(path.to_path_buf());
        Ok(())
    }
}

impl Default for NativeBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionBackend for NativeBackend {
    fn reset(&mut self, config: &MachineConfig) -> Result<()> {
        let ram = u32::try_from(config.ram_bytes).map_err(|_| {
            X86Error::BackendUnavailable(format!(
                "native v86 core supports guest RAM up to 4 GiB; requested {} bytes",
                config.ram_bytes
            ))
        })?;
        let vga = u32::try_from(config.vga_memory_bytes).map_err(|_| {
            X86Error::BackendUnavailable(format!(
                "native v86 core supports VGA memory up to 4 GiB; requested {} bytes",
                config.vga_memory_bytes
            ))
        })?;
        let mut cpu = NativeCpu::new(ram, vga);
        if let Some(path) = &self.ninep_root {
            cpu.set_9p_root(path)
                .map_err(X86Error::BackendUnavailable)?;
        }
        self.cpu = Some(cpu);
        Ok(())
    }

    fn restore_state(&mut self, state: &SavedState) -> Result<()> {
        let cpu = self.cpu.as_mut().ok_or_else(|| {
            X86Error::BackendUnavailable("native backend must be reset before restore".to_owned())
        })?;
        let (state_object, buffers) = state.cpu_state_and_buffers()?;
        cpu.restore_v86_state(&state_object, &buffers)
            .map_err(X86Error::InvalidState)
    }

    fn step(&mut self) -> Result<bool> {
        let cpu = self.cpu.as_mut().ok_or_else(|| {
            X86Error::BackendUnavailable("native backend is not prepared".to_owned())
        })?;
        let _executed = cpu.step(self.instructions_per_step);
        // HLT is an interruptible guest idle state. It is not a terminal
        // machine condition, so the outer run loop must keep polling timers.
        Ok(false)
    }

    fn read_memory(&self, address: u64, buffer: &mut [u8]) -> Result<()> {
        let cpu = self.cpu.as_ref().ok_or_else(|| {
            X86Error::BackendUnavailable("native backend is not prepared".to_owned())
        })?;
        let address = u32::try_from(address).map_err(|_| {
            X86Error::InvalidImage("guest memory address exceeds 32-bit x86 range".to_owned())
        })?;
        if cpu.read_memory(address, buffer) {
            Ok(())
        } else {
            Err(X86Error::InvalidImage(
                "guest memory read is out of bounds".to_owned(),
            ))
        }
    }

    fn write_memory(&mut self, address: u64, data: &[u8]) -> Result<()> {
        let cpu = self.cpu.as_mut().ok_or_else(|| {
            X86Error::BackendUnavailable("native backend is not prepared".to_owned())
        })?;
        let address = u32::try_from(address).map_err(|_| {
            X86Error::InvalidImage("guest memory address exceeds 32-bit x86 range".to_owned())
        })?;
        if cpu.write_memory(address, data) {
            Ok(())
        } else {
            Err(X86Error::InvalidImage(
                "guest memory write is out of bounds".to_owned(),
            ))
        }
    }

    fn vga_framebuffer_rgb(&self) -> Option<(u32, u32, Vec<u8>)> {
        self.cpu.as_ref()?.vga_framebuffer_rgb()
    }

    fn inject_text(&mut self, text: &str) -> Result<usize> {
        if self.cpu.is_none() {
            return Err(X86Error::BackendUnavailable(
                "native backend is not prepared".to_owned(),
            ));
        }
        Ok(native_v86_core::native_runtime::inject_keyboard_text(text))
    }
}
