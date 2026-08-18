# Приклади завантаження образів

## Локальні ресурси

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

## URL bootloader

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

## Перевірка checksum

```rust,no_run
use x86::{Image, ImageKind};

fn main() -> x86::Result<()> {
    let image = Image::from_file(ImageKind::RawDisk, "disk.img")?;
    println!("sha256 = {}", image.sha256());
    image.verify_sha256("expected-sha256")?;
    Ok(())
}
```

## State

```rust,no_run
use x86::SavedState;

fn main() -> x86::Result<()> {
    let state = SavedState::from_file("machine-state.bin.zst")?;
    println!("summary = {:?}", state.summary());
    Ok(())
}
```

Англійський варіант із додатковими прикладами доступний у [Image loading examples](../examples/images.md).
