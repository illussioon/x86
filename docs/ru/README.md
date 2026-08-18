# Документация x86-native на русском

`x86-native` — нативный Rust-пакет для построения host-слоя x86-машины. Имя пакета в crates.io — `x86-native`, а имя импортируемой библиотеки — `x86`.

## Установка

```toml
[dependencies]
x86-native = "0.1"
```

Или:

```bash
cargo add x86-native
```

Пакет собирается обычным Cargo под native targets и не требует браузера или WebAssembly runtime. По умолчанию включены `remote` для нативной загрузки HTTP(S) и `zstd` для сжатых state-файлов. Полностью локальная сборка выполняется так:

```bash
cargo build --no-default-features
```

## Быстрый старт

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

## Образы и загрузчик

`Image::from_file` и `Image::from_bytes` работают с файлами и memory buffers. `Bootloader::load` принимает `Resource::file`, `Resource::bytes` или `Resource::url`. URL-загрузка выполняется нативным Rust-кодом; браузер, DOM и WebAssembly не используются.

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

`SavedState` проверяет заголовок и metadata v86 state version 6. Поддерживаются несжатые state-файлы и Zstandard-сжатие при включённой feature `zstd`.

```rust,no_run
use x86::SavedState;

fn main() -> x86::Result<()> {
    let state = SavedState::from_file("machine-state.bin.zst")?;
    let summary = state.summary();
    println!("version={} buffers={} sha256={}", summary.version, summary.buffer_count, summary.sha256);
    Ok(())
}
```

## Жизненный цикл машины

`Machine` хранит конфигурацию и подключённые ресурсы. `ExecutionBackend` является явной точкой расширения для native CPU, памяти и устройств. Если backend не подключён, библиотека возвращает `BackendUnavailable`, а не сообщает, что гостевая система уже исполняется.

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

Доступны команды `help`, `capabilities`, `load bios <path>`, `load disk <path>`, `load state <path>`, `load bootloader <path|url>`, `info`, `checksum state`, `prepare`, `run` и `quit`.

Продолжайте с разделов [обзор API](../ru/api.md), [примеры образов](../ru/examples.md), [консоль](../ru/console.md), [архитектура](../ru/architecture.md) и [релизы](../ru/releases.md).
