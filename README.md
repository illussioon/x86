# x86-native

[![Crates.io](https://img.shields.io/crates/v/x86-native.svg)](https://crates.io/crates/x86-native)
[![Documentation](https://img.shields.io/docsrs/x86-native)](https://docs.rs/x86-native)
[![CI](https://github.com/illussioon/x86/actions/workflows/ci.yml/badge.svg)](https://github.com/illussioon/x86/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/illussioon/x86)](https://github.com/illussioon/x86/releases)
[![License](https://img.shields.io/badge/license-BSD--2--Clause%20OR%20MIT-blue.svg)](LICENSE)

**x86-native** is a cross-platform native Rust library for building x86 machine hosts without a browser, DOM, WebAssembly runtime or web frontend. The package name on crates.io is `x86-native`; the Rust library target remains `x86`, so applications use `use x86::...`.

> **Project status:** the native interpreter, saved-state restore, VirtIO-9P transport, host-directory filesystem, VGA framebuffer extraction and PS/2 text input are implemented and tested. The Arch saved state reaches the restored `root@localhost:~#` shell; `dump-screen` captures the guest framebuffer and `type` sends keyboard text. The project is still an incremental native device port, not a claim that every optional upstream v86 peripheral is already implemented.

## Languages

| Language | Documentation |
| --- | --- |
| English | [English guide](docs/en/README.md) |
| Русский | [Русская документация](docs/ru/README.md) |
| Українська | [Українська документація](docs/uk/README.md) |

The [documentation index](docs/README.md) contains the API map, examples, architecture diagrams, console screenshots and release notes.

## Install from Cargo

Add the package to your application:

```toml
[dependencies]
x86-native = "0.1"
```

Then import the library target as `x86`:

```rust
use x86::{Image, ImageKind, Machine, MachineConfig};

fn main() -> x86::Result<()> {
    let mut machine = Machine::new(
        MachineConfig::default().with_ram_bytes(512 * 1024 * 1024),
    );

    machine.set_disk(Image::from_file(ImageKind::RawDisk, "disk.img")?)?;
    println!("machine status: {:?}", machine.status());
    Ok(())
}
```

The package is designed for stable Rust and native targets supported by Cargo. Run `cargo add x86-native` or edit `Cargo.toml` manually as shown above.

## Features

| Feature | Default | Purpose |
| --- | ---: | --- |
| `remote` | yes | Native HTTP(S) resource loading through Rust networking code. It does not open a browser. |
| `zstd` | yes | Decode Zstandard-compressed saved states. |
| `--no-default-features` | no | Offline/local-only build with no remote loader and no zstd decoder. |

For a strictly offline build:

```bash
cargo build --no-default-features
```

## Main API

The `Image` type represents BIOS, VGA BIOS, raw disks, ISO images, kernels, initrds, bootloaders and memory-backed resources. It supports local file loading, SHA-256 calculation and checksum verification.

`Resource` and `Bootloader` provide a single interface for local paths, in-memory bytes and optional HTTP(S) URLs. `SavedState` validates v86-compatible state headers, metadata, buffer counts, memory size and compressed state data.

`MachineConfig` describes RAM, VGA memory, CPU frequency hints, command line and console mode. `Machine` attaches the machine resources and exposes `prepare`, `run` and `stop`. `ExecutionBackend` is a platform-neutral trait for connecting the actual native CPU/device engine.

```rust,no_run
use x86::{Bootloader, Image, ImageKind, Machine, MachineConfig, Resource, SavedState};

fn main() -> x86::Result<()> {
    let mut machine = Machine::new(
        MachineConfig::default()
            .with_ram_bytes(512 * 1024 * 1024)
            .with_command_line("rw console=ttyS0"),
    );

    machine.set_bios(Image::from_file(ImageKind::Bios, "seabios.bin")?)?;
    machine.set_vga_bios(Image::from_file(ImageKind::VgaBios, "vgabios.bin")?)?;
    machine.set_disk(Image::from_file(ImageKind::RawDisk, "disk.img")?)?;
    machine.set_saved_state(SavedState::from_file("state.bin.zst")?);
    machine.set_bootloader(Bootloader::load(Resource::url(
        "https://example.org/bootloader.bin",
    ))?);

    println!("state: {:?}", machine.status());
    Ok(())
}
```

## Native console

Build and launch the terminal application:

```bash
cargo run --bin x86-console
```

The console is a normal native process. It does not start a web server or require a browser:

```text
x86> load bios seabios.bin
x86> load vga-bios vgabios.bin
x86> load disk disk.img
x86> load state arch_state-v3.bin.zst
x86> load bootloader https://example.org/bootloader.bin
x86> info
x86> load state ./image/arch_state-v3.bin.zst
x86> run-state 1000000
x86> dump-screen ./arch-screen.ppm
x86> type echo native-ok
x86> run-state 1000000
x86> dump-screen ./arch-screen-after-input.ppm
x86> quit
```

The native console also supports `run-state`, `dump-screen <path.ppm>` and `type <text>`. `dump-screen` writes a standard PPM framebuffer that can be opened by macOS Preview, ImageMagick or converted to PNG; `type` injects PC AT keyboard scancodes and appends Enter. `prepare` and `run` return a typed `BackendUnavailable` error until an `ExecutionBackend` is attached. This is intentional: the library never reports a guest as running when no CPU/device backend exists.

## Architecture

![x86-native architecture](docs/assets/architecture.png)

The source diagram is available as [`docs/assets/architecture.mmd`](docs/assets/architecture.mmd). The host layer is platform-neutral; platform-specific console, filesystem and networking adapters remain outside the core API.

## Releases

The [GitHub Releases page](https://github.com/illussioon/x86/releases) contains versioned native artifacts and source packages. The release workflow is configured to build Linux, macOS Intel, macOS Apple Silicon and Windows artifacts when a version tag is pushed.

| Target | Typical artifact | Build target |
| --- | --- | --- |
| Linux x86_64 | `x86-console-linux-x86_64`, `libx86-linux-x86_64.so` | `x86_64-unknown-linux-gnu` |
| macOS Intel | `x86-console-macos-x86_64`, `libx86-macos-x86_64.dylib` | `x86_64-apple-darwin` |
| macOS Apple Silicon | `x86-console-macos-aarch64`, `libx86-macos-aarch64.dylib` | `aarch64-apple-darwin` |
| Windows x86_64 | `x86-console-windows-x86_64.exe`, `x86-windows-x86_64.dll` | `x86_64-pc-windows-msvc` |

## Build from source

```bash
git clone https://github.com/illussioon/x86.git
cd x86
cargo check
cargo test
cargo package
cargo run --bin x86-console
```

For native release builds:

```bash
cargo build --release
```

## License

Licensed under either of [BSD-2-Clause](LICENSE) or [MIT](LICENSE.MIT), at your option.

## References

The API follows standard Cargo package conventions [1] and uses the repository's native Rust implementation as the source of truth [2].

[1]: https://doc.rust-lang.org/cargo/
[2]: https://github.com/illussioon/x86
