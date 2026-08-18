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

    let bootloader = Bootloader::load(Resource::url("https://example.org/bootloader.bin"))?;
    machine.set_bootloader(bootloader);

    if let Some(state) = machine.saved_state() {
        println!("saved state: {:?}", state.summary());
    }
    println!("machine status: {:?}", machine.status());
    Ok(())
}
