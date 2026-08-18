# x86

`x86` is a cross-platform native Rust library for hosting an x86 machine implementation. The crate is designed for Linux, macOS, Windows, BSD and other Rust-supported native targets. It has no browser, DOM or WebAssembly runtime dependency.

## Manifest

The package manifest includes `name`, `version`, `authors`, `description`, `license`, keywords and a placeholder for the repository URL. Replace the commented `repository` line in `Cargo.toml` with your repository before publishing.

## Features

The public API covers the native resources needed by an x86 machine host. `Image` represents BIOS, VGA BIOS, raw disks, ISO images, kernels, initrds, bootloaders and other byte resources. `Resource` loads data from memory, a local path or, when the default `remote` feature is enabled, HTTP(S). `SavedState` validates and reads v86-compatible version-6 state files, including Zstandard-compressed files when the default `zstd` feature is enabled. `MachineConfig` and `Machine` collect resources and expose a platform-neutral lifecycle. `ExecutionBackend` is the native CPU/device boundary that the actual emulator core will implement.

> The current package is the native library and host API layer. It deliberately does not pretend that a metadata loader is a complete x86 CPU/device emulator. An actual execution backend must implement CPU, memory, interrupts, timers, VGA, storage, network and other devices behind `ExecutionBackend`.

## Rust API example

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

    let bootloader = Bootloader::load(Resource::url(
        "https://example.org/bootloader.bin",
    ))?;
    machine.set_bootloader(bootloader);

    println!("state: {:?}", machine.status());
    Ok(())
}
```

For an offline-only build, disable both optional capabilities:

```bash
cargo build --no-default-features
```

In that mode the library still supports memory and local-file resources, but URL loading and `.zst` state decoding return explicit typed errors instead of requiring a network or decompression backend.

## Native console

The `x86-console` binary is a normal terminal process. It does not open a browser or start a web server:

```bash
cargo run --bin x86-console
```

Inside the console:

```text
load bios seabios.bin
load vga-bios vgabios.bin
load disk disk.img
load state arch_state-v3.bin.zst
load bootloader https://example.org/bootloader.bin
info
checksum state
prepare
quit
```

`prepare` and `run` require an `ExecutionBackend` implementation. Without one, the console reports a typed `BackendUnavailable` error rather than silently claiming that a guest was executed.

## Platform builds

The crate uses stable Rust and platform-neutral standard library APIs. The normal commands are:

```bash
cargo check
cargo test
cargo build --release
```

For Windows from a Windows host, use `cargo build --release`. For macOS, use the native Apple target supplied by `rustup`; for Linux, use the native Linux target. Cross-compilation can use any Rust target supported by the installed toolchain.

## License and repository

The default license expression is `BSD-2-Clause OR MIT`. Add the project repository URL to `Cargo.toml` when you decide where to publish it.
