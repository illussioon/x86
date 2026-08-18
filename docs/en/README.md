# x86-native documentation

`x86-native` is a native Rust package for composing an x86 machine host. The package name is `x86-native`, while the exported Rust library target is `x86`.

## Installation

```toml
[dependencies]
x86-native = "0.1"
```

Or run:

```bash
cargo add x86-native
```

The crate builds on native Cargo targets and does not require a browser or WebAssembly runtime. The default features are `remote` for native HTTP(S) loading and `zstd` for compressed state files. Use `cargo build --no-default-features` for an offline/local-only build.

## Quick start

```rust,no_run
use x86::{Image, ImageKind, Machine, MachineConfig};

fn main() -> x86::Result<()> {
    let mut machine = Machine::new(
        MachineConfig::default().with_ram_bytes(256 * 1024 * 1024),
    );
    machine.set_bios(Image::from_file(ImageKind::Bios, "seabios.bin")?)?;
    machine.set_disk(Image::from_file(ImageKind::RawDisk, "disk.img")?)?;
    println!("status = {:?}", machine.status());
    Ok(())
}
```

## Resource loading

`Image::from_file` and `Image::from_bytes` handle local and memory resources. `Bootloader::load` accepts `Resource::file`, `Resource::bytes` or `Resource::url`. The URL path is native Rust networking and is not a web frontend.

```rust,no_run
use x86::{Bootloader, Resource};

fn main() -> x86::Result<()> {
    let local = Bootloader::load(Resource::file("bootloader.bin"))?;
    let remote = Bootloader::load(Resource::url(
        "https://example.org/bootloader.bin",
    ))?;
    println!("{} {}", local.image.len(), remote.image.len());
    Ok(())
}
```

## Saved states

`SavedState` supports the v86 version-6 header and metadata format. It can read an uncompressed state or a Zstandard-compressed state when the `zstd` feature is active.

```rust,no_run
use x86::SavedState;

fn main() -> x86::Result<()> {
    let state = SavedState::from_file("machine-state.bin.zst")?;
    let summary = state.summary();
    println!("version={} buffers={} sha256={}", summary.version, summary.buffer_count, summary.sha256);
    Ok(())
}
```

## Machine lifecycle

`Machine` owns configuration and attached resources. `ExecutionBackend` is intentionally explicit: it is the extension point for a native CPU, memory and device implementation. A machine without a backend returns `BackendUnavailable` instead of claiming to execute a guest.

```rust,no_run
use x86::{Machine, MachineConfig, RunOptions};

fn main() -> x86::Result<()> {
    let mut machine = Machine::new(MachineConfig::default());
    // machine.attach_backend(my_native_backend);
    machine.prepare()?;
    let report = machine.run(RunOptions::default())?;
    println!("steps={} halted={}", report.steps, report.halted);
    Ok(())
}
```

## Console

```bash
cargo run --bin x86-console
```

The console accepts `help`, `capabilities`, `load bios <path>`, `load disk <path>`, `load state <path>`, `load bootloader <path|url>`, `info`, `checksum state`, `prepare`, `run` and `quit`.

Continue with [API overview](../api/overview.md), [image examples](../examples/images.md), [console examples](../examples/console.md), [architecture](architecture.md) and [releases](releases.md).
