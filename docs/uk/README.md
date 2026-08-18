# Документація x86-native українською

`x86-native` — нативний Rust-пакет для створення host-рівня x86-машини. Назва пакета в crates.io — `x86-native`, а назва бібліотеки для імпорту — `x86`.

## Встановлення

```toml
[dependencies]
x86-native = "0.1"
```

Або виконайте:

```bash
cargo add x86-native
```

Пакет збирається звичайним Cargo для native targets і не потребує браузера або WebAssembly runtime. За замовчуванням увімкнено `remote` для нативного HTTP(S)-завантаження та `zstd` для стиснутих state-файлів. Повністю локальна збірка:

```bash
cargo build --no-default-features
```

## Швидкий старт

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

## Образи та bootloader

`Image::from_file` і `Image::from_bytes` працюють із файлами та memory buffers. `Bootloader::load` приймає `Resource::file`, `Resource::bytes` або `Resource::url`. URL-завантаження виконується нативним Rust-кодом; браузер, DOM і WebAssembly не використовуються.

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

## Saved state

`SavedState` перевіряє заголовок і metadata v86 state version 6. Підтримуються нестиснуті state-файли та Zstandard-стиснення, якщо увімкнено feature `zstd`.

```rust,no_run
use x86::SavedState;

fn main() -> x86::Result<()> {
    let state = SavedState::from_file("machine-state.bin.zst")?;
    let summary = state.summary();
    println!("version={} buffers={} sha256={}", summary.version, summary.buffer_count, summary.sha256);
    Ok(())
}
```

## Життєвий цикл машини

`Machine` зберігає конфігурацію та підключені ресурси. `ExecutionBackend` є явною точкою розширення для native CPU, пам’яті та пристроїв. Якщо backend не підключено, бібліотека повертає `BackendUnavailable`, а не повідомляє, що гостьова система вже виконується.

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

## Консоль

```bash
cargo run --bin x86-console
```

Доступні команди `help`, `capabilities`, `load bios <path>`, `load disk <path>`, `load state <path>`, `load bootloader <path|url>`, `info`, `checksum state`, `prepare`, `run` і `quit`.

Продовжуйте з розділів [огляд API](../uk/api.md), [приклади образів](../uk/examples.md), [консоль](../uk/console.md), [архітектура](../uk/architecture.md) та [релізи](../uk/releases.md).
