# Обзор API

Библиотека состоит из пяти платформенно-независимых модулей.

| Модуль | Основные типы | Назначение |
| --- | --- | --- |
| `image` | `Image`, `ImageKind` | Хранение образа, определение типа, SHA-256 и проверка целостности. |
| `bootloader` | `Resource`, `Bootloader`, `FetchOptions` | Загрузка из файла, памяти и optional HTTP(S). |
| `state` | `SavedState`, `StateHeader`, `StateSummary` | Проверка и анализ v86 state version 6, включая optional Zstandard. |
| `machine` | `Machine`, `MachineConfig`, `ExecutionBackend` | Подключение ресурсов и native execution boundary. |
| `error` | `X86Error`, `Result` | Типизированные ошибки I/O, формата, сети и backend. |

## Image

`Image::from_bytes` используется для memory buffers, а `Image::from_file` — для локальных файлов. Методы `bytes`, `len`, `kind`, `name` и `source` предоставляют данные образа; `sha256` и `verify_sha256` позволяют проверять его целостность.

## Resource и Bootloader

`Resource` имеет варианты `File`, `Url` и `Bytes`. `Bootloader::load` использует стандартные настройки, а `Bootloader::load_with_options` позволяет задать timeout и user-agent. HTTP(S)-загрузка является нативной Rust-функцией и включается feature `remote`. Для полностью локальной сборки используйте `--no-default-features`.

## SavedState

`SavedState::from_bytes` и `SavedState::from_file` проверяют magic, version, длину metadata и наличие полей `state`/`buffer_infos`. Метод `summary` возвращает размеры encoded/decoded data, compression status, количество buffers, размер памяти и SHA-256.

## Machine

`MachineConfig` поддерживает builder-методы `with_ram_bytes`, `with_vga_memory_bytes`, `with_cpu_hz`, `with_command_line` и `with_console`. Основные методы ресурсов:

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

`prepare` сбрасывает подключённый backend, `run` выполняет шаги через `RunOptions`, а `stop` переводит машину в состояние `Stopped`.

## ExecutionBackend

Backend реализует четыре метода: `reset`, `step`, `read_memory` и `write_memory`. Это отдельная native Rust boundary для CPU, памяти, прерываний и устройств. Если backend не подключён, библиотека возвращает `X86Error::BackendUnavailable`.

## Ошибки

Все операции возвращают `x86::Result<T>`. Приложение может сопоставлять варианты `X86Error`, не разбирая текст ошибок вручную.
