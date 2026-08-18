# Image loading examples

## Local resources

```rust,no_run
use x86::{Image, ImageKind, Machine, MachineConfig};

fn main() -> x86::Result<()> {
    let mut machine = Machine::new(MachineConfig::default());
    machine.set_bios(Image::from_file(ImageKind::Bios, "seabios.bin")?)?;
    machine.set_vga_bios(Image::from_file(ImageKind::VgaBios, "vgabios.bin")?)?;
    machine.set_disk(Image::from_file(ImageKind::RawDisk, "disk.img")?)?;
    machine.set_cdrom(Image::from_file(ImageKind::Iso9660, "installer.iso")?)?;
    Ok(())
}
```

## Memory resources

```rust,no_run
use x86::{Image, ImageKind, Machine, MachineConfig};

fn main() -> x86::Result<()> {
    let mut machine = Machine::new(MachineConfig::default());
    let bios = Image::from_bytes(ImageKind::Bios, "generated-bios", vec![0u8; 64 * 1024]);
    machine.set_bios(bios)?;
    Ok(())
}
```

## Native URL bootloader

```rust,no_run
use x86::{Machine, MachineConfig, Resource};

fn main() -> x86::Result<()> {
    let mut machine = Machine::new(MachineConfig::default());
    machine.load_bootloader(Resource::url(
        "https://example.org/bootloader.bin",
    ))?;
    Ok(())
}
```

## Integrity checks

```rust,no_run
use x86::{Image, ImageKind};

fn main() -> x86::Result<()> {
    let image = Image::from_file(ImageKind::RawDisk, "disk.img")?;
    println!("sha256 = {}", image.sha256());
    image.verify_sha256("put-the-expected-sha256-here")?;
    Ok(())
}
```

## Saved state

```rust,no_run
use x86::{Machine, MachineConfig, SavedState};

fn main() -> x86::Result<()> {
    let mut machine = Machine::new(MachineConfig::default());
    let state = SavedState::from_file("machine-state.bin.zst")?;
    println!("summary = {:?}", state.summary());
    machine.set_saved_state(state);
    Ok(())
}
```
