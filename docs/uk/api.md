# Огляд API

Бібліотека складається з п’яти платформно-незалежних модулів.

| Модуль | Основні типи | Призначення |
| --- | --- | --- |
| `image` | `Image`, `ImageKind` | Зберігання образу, визначення типу, SHA-256 і перевірка цілісності. |
| `bootloader` | `Resource`, `Bootloader`, `FetchOptions` | Завантаження з файлу, пам’яті та optional HTTP(S). |
| `state` | `SavedState`, `StateHeader`, `StateSummary` | Перевірка й аналіз v86 state version 6, з optional Zstandard. |
| `machine` | `Machine`, `MachineConfig`, `ExecutionBackend` | Підключення ресурсів і native execution boundary. |
| `error` | `X86Error`, `Result` | Типізовані помилки I/O, формату, мережі та backend. |

## Image

`Image::from_bytes` використовується для memory buffers, а `Image::from_file` — для локальних файлів. Методи `bytes`, `len`, `kind`, `name` і `source` надають дані образу; `sha256` та `verify_sha256` перевіряють його цілісність.

## Resource і Bootloader

`Resource` має варіанти `File`, `Url` і `Bytes`. `Bootloader::load` використовує стандартні налаштування, а `Bootloader::load_with_options` дає змогу задати timeout і user-agent. HTTP(S)-завантаження є нативною Rust-функцією та вмикається feature `remote`. Для повністю локальної збірки використовуйте `--no-default-features`.

## SavedState

`SavedState::from_bytes` і `SavedState::from_file` перевіряють magic, version, довжину metadata та наявність полів `state`/`buffer_infos`. Метод `summary` повертає розміри encoded/decoded data, compression status, кількість buffers, розмір пам’яті та SHA-256.

## Machine

`MachineConfig` підтримує builder-методи `with_ram_bytes`, `with_vga_memory_bytes`, `with_cpu_hz`, `with_command_line` і `with_console`. Основні методи ресурсів:

```text
set_bios
set_vga_bios
set_disk
set_cdrom
set_bootloader
load_bootloader
load_image
set_saved_state
load_saved_state
```

`prepare` скидає підключений backend, `run` виконує кроки через `RunOptions`, а `stop` переводить машину у стан `Stopped`.

## ExecutionBackend

Backend реалізує чотири методи: `reset`, `step`, `read_memory` і `write_memory`. Це окрема native Rust boundary для CPU, пам’яті, переривань і пристроїв. Якщо backend не підключений, бібліотека повертає `X86Error::BackendUnavailable`.

## Помилки

Усі операції повертають `x86::Result<T>`. Застосунок може зіставляти варіанти `X86Error`, не розбираючи текст помилок вручну.
