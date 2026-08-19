use std::io::{self, BufRead, Write};
use std::path::PathBuf;

#[cfg(feature = "native-runtime")]
use x86::NativeBackend;
use x86::{Bootloader, Image, ImageKind, Machine, MachineConfig, Resource, SavedState, X86Error};

fn print_help() {
    println!(
        "Commands:\n  help                         Show this help\n  capabilities                Show native capabilities\n  info                        Show machine configuration and attached images\n  config ram <bytes>          Set guest RAM size\n  config vga <bytes>          Set VGA memory size\n  load bios <path>             Load BIOS image\n  load vga-bios <path>         Load VGA BIOS image\n  load disk <path>             Load raw disk image\n  load cdrom <path>            Load ISO/CD-ROM image\n  load state <path>            Load v86 saved state (.bin or .bin.zst)\n  load bootloader <path|url>   Load a bootloader from disk or HTTP(S)\n  checksum <kind>              Print SHA-256 for bios/vga-bios/disk/cdrom/bootloader/state\n  prepare                     Validate backend readiness\n  run                         Run the attached native backend\n  run-state [max_steps]       Restore and run the loaded v86 saved state\n  dump-screen <path.ppm>      Save the current guest VGA framebuffer as PPM\n  type <text>                 Send text and Enter to the guest keyboard\n  quit                        Exit"
    );
}

fn print_image(label: &str, image: Option<&Image>) {
    match image {
        Some(image) => println!(
            "{label}: {} ({} bytes, {:?}, sha256={})",
            image.name(),
            image.len(),
            image.kind(),
            image.sha256()
        ),
        None => println!("{label}: <not loaded>"),
    }
}

fn print_info(machine: &Machine) {
    let config = machine.config();
    println!("status: {:?}", machine.status());
    println!("ram: {} bytes", config.ram_bytes);
    println!("vga memory: {} bytes", config.vga_memory_bytes);
    println!("cpu frequency hint: {} Hz", config.cpu_hz);
    println!(
        "console: {:?} {}x{}",
        config.console.mode, config.console.width, config.console.height
    );
    println!(
        "command line: {}",
        config.command_line.as_deref().unwrap_or("<none>")
    );
    print_image("bios", machine.bios());
    print_image("vga-bios", machine.vga_bios());
    print_image("disk", machine.disk());
    print_image("cdrom", machine.cdrom());
    if let Some(bootloader) = machine.bootloader() {
        print_image("bootloader", Some(&bootloader.image));
    } else {
        println!("bootloader: <not loaded>");
    }
    if let Some(state) = machine.saved_state() {
        let summary = state.summary();
        println!(
            "saved-state: version {}, compressed={}, {} buffers, {} decoded bytes, sha256={}",
            summary.version,
            summary.compressed,
            summary.buffer_count,
            summary.decoded_bytes,
            summary.sha256
        );
    } else {
        println!("saved-state: <not loaded>");
    }
}

fn parse_u64(value: Option<&str>) -> Result<u64, X86Error> {
    value
        .ok_or_else(|| X86Error::InvalidImage("missing numeric value".to_owned()))?
        .parse::<u64>()
        .map_err(|error| X86Error::InvalidImage(format!("invalid number: {error}")))
}

fn load_file(machine: &mut Machine, kind: ImageKind, path: &str) -> Result<(), X86Error> {
    let image = Image::from_file(kind, PathBuf::from(path))?;
    match kind {
        ImageKind::Bios => machine.set_bios(image),
        ImageKind::VgaBios => machine.set_vga_bios(image),
        ImageKind::RawDisk => machine.set_disk(image),
        ImageKind::Iso9660 => machine.set_cdrom(image),
        _ => Err(X86Error::InvalidImage(
            "unsupported console image kind".to_owned(),
        )),
    }
}

fn print_checksum(machine: &Machine, kind: &str) {
    let checksum = match kind {
        "bios" => machine.bios().map(Image::sha256),
        "vga-bios" => machine.vga_bios().map(Image::sha256),
        "disk" => machine.disk().map(Image::sha256),
        "cdrom" => machine.cdrom().map(Image::sha256),
        "bootloader" => machine.bootloader().map(|x| x.image.sha256()),
        "state" => machine.saved_state().map(SavedState::sha256),
        _ => None,
    };
    match checksum {
        Some(value) => println!("{kind}: {value}"),
        None => println!("{kind}: not loaded or unknown kind"),
    }
}

fn dump_screen(machine: &Machine, path: &str) -> Result<(), X86Error> {
    let (width, height, pixels) = machine.vga_framebuffer_rgb().ok_or_else(|| {
        X86Error::BackendUnavailable(
            "guest is not currently exposing a 32-bit graphical VGA framebuffer".to_owned(),
        )
    })?;
    let expected = width as usize * height as usize * 3;
    if pixels.len() != expected {
        return Err(X86Error::InvalidImage(
            "invalid framebuffer length".to_owned(),
        ));
    }
    let mut output = Vec::with_capacity(expected + 32);
    output.extend_from_slice(format!("P6\n{} {}\n255\n", width, height).as_bytes());
    output.extend_from_slice(&pixels);
    std::fs::write(path, output).map_err(|source| X86Error::Io {
        path: PathBuf::from(path),
        source,
    })?;
    println!("screen saved: {} ({}x{})", path, width, height);
    Ok(())
}

fn main() -> Result<(), X86Error> {
    let mut machine = Machine::new(MachineConfig::default());
    #[cfg(feature = "native-runtime")]
    {
        let backend = match std::env::var_os("X86_9P_ROOT") {
            Some(root) => NativeBackend::new().with_9p_root(PathBuf::from(root)),
            None => NativeBackend::new(),
        };
        machine.attach_backend(backend);
    }
    println!("x86 native console v{}", x86::API_VERSION);
    println!("No browser or WebAssembly runtime is used. Type `help` for commands.");
    print!("x86> ");
    io::stdout().flush().map_err(|source| X86Error::Io {
        path: PathBuf::from("stdout"),
        source,
    })?;

    for line in io::stdin().lock().lines() {
        let line = line.map_err(|source| X86Error::Io {
            path: PathBuf::from("stdin"),
            source,
        })?;
        let mut parts = line.split_whitespace();
        let command = parts.next().unwrap_or_default();
        let result = match command {
            "" => Ok(()),
            "help" => {
                print_help();
                Ok(())
            }
            "capabilities" => {
                for capability in x86::native_capabilities() {
                    println!("- {capability}");
                }
                Ok(())
            }
            "info" => {
                print_info(&machine);
                Ok(())
            }
            "config" => match parts.next() {
                Some("ram") => {
                    machine.set_ram_bytes(parse_u64(parts.next())?);
                    println!("RAM configuration updated");
                    Ok(())
                }
                Some("vga") => {
                    machine.set_vga_memory_bytes(parse_u64(parts.next())?);
                    println!("VGA memory configuration updated");
                    Ok(())
                }
                _ => Err(X86Error::InvalidImage(
                    "usage: config ram|vga <bytes>".to_owned(),
                )),
            },
            "load" => match parts.next() {
                Some("bios") => load_file(
                    &mut machine,
                    ImageKind::Bios,
                    parts
                        .next()
                        .ok_or_else(|| X86Error::InvalidImage("missing BIOS path".to_owned()))?,
                ),
                Some("vga-bios") => load_file(
                    &mut machine,
                    ImageKind::VgaBios,
                    parts.next().ok_or_else(|| {
                        X86Error::InvalidImage("missing VGA BIOS path".to_owned())
                    })?,
                ),
                Some("disk") => load_file(
                    &mut machine,
                    ImageKind::RawDisk,
                    parts
                        .next()
                        .ok_or_else(|| X86Error::InvalidImage("missing disk path".to_owned()))?,
                ),
                Some("cdrom") => load_file(
                    &mut machine,
                    ImageKind::Iso9660,
                    parts
                        .next()
                        .ok_or_else(|| X86Error::InvalidImage("missing CD-ROM path".to_owned()))?,
                ),
                Some("state") => {
                    let path = parts
                        .next()
                        .ok_or_else(|| X86Error::InvalidImage("missing state path".to_owned()))?;
                    machine.load_saved_state(Resource::file(path))?;
                    println!("saved state loaded; RAM configuration updated from state");
                    Ok(())
                }
                Some("bootloader") => {
                    let source = parts.next().ok_or_else(|| {
                        X86Error::InvalidImage("missing bootloader path or URL".to_owned())
                    })?;
                    machine.set_bootloader(Bootloader::load(
                        if source.starts_with("http://") || source.starts_with("https://") {
                            Resource::url(source)
                        } else {
                            Resource::file(source)
                        },
                    )?);
                    println!("bootloader loaded");
                    Ok(())
                }
                _ => Err(X86Error::InvalidImage(
                    "usage: load bios|vga-bios|disk|cdrom|state|bootloader <source>".to_owned(),
                )),
            },
            "checksum" => {
                print_checksum(&machine, parts.next().unwrap_or_default());
                Ok(())
            }
            "dump-screen" => {
                let path = parts.next().ok_or_else(|| {
                    X86Error::InvalidImage("usage: dump-screen <path.ppm>".to_owned())
                })?;
                dump_screen(&machine, path)
            }
            "type" => {
                let text = parts.collect::<Vec<_>>().join(" ");
                let count = machine.inject_text(&format!("{text}\n"))?;
                println!("keyboard input queued: {count} characters");
                Ok(())
            }
            "prepare" => match machine.prepare() {
                Ok(()) => {
                    println!("machine is ready");
                    Ok(())
                }
                Err(error) => {
                    println!("machine is not ready: {error}");
                    Ok(())
                }
            },
            "run" | "run-state" => {
                if command == "run-state" && machine.saved_state().is_none() {
                    Err(X86Error::InvalidState(
                        "load a v86 saved state before run-state".to_owned(),
                    ))
                } else {
                    let max_steps = if command == "run-state" {
                        parts
                            .next()
                            .map(|value| {
                                value.parse::<u64>().map_err(|error| {
                                    X86Error::InvalidImage(format!("invalid max_steps: {error}"))
                                })
                            })
                            .transpose()?
                            .or(Some(100_000))
                    } else {
                        None
                    };
                    let options = x86::RunOptions {
                        max_steps,
                        ..Default::default()
                    };
                    match machine.run(options) {
                        Ok(report) => {
                            println!(
                                "run finished: {} steps, halted={}",
                                report.steps, report.halted
                            );
                            Ok(())
                        }
                        Err(error) => {
                            println!("run unavailable: {error}");
                            Ok(())
                        }
                    }
                }
            }
            "quit" | "exit" => break,
            _ => Err(X86Error::InvalidImage(
                "unknown command; type `help`".to_owned(),
            )),
        };
        if let Err(error) = result {
            println!("error: {error}");
        }
        print!("x86> ");
        io::stdout().flush().map_err(|source| X86Error::Io {
            path: PathBuf::from("stdout"),
            source,
        })?;
    }
    Ok(())
}
