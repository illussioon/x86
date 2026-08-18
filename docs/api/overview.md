# API overview

The library is split into five platform-neutral modules.

| Module | Main types | Responsibility |
| --- | --- | --- |
| `image` | `Image`, `ImageKind` | Own image bytes, identify the resource kind, calculate and verify SHA-256. |
| `bootloader` | `Resource`, `Bootloader`, `FetchOptions` | Resolve local, memory and optional HTTP(S) sources. |
| `state` | `SavedState`, `StateHeader`, `StateSummary` | Validate and inspect v86 state version 6, including optional Zstandard decoding. |
| `machine` | `Machine`, `MachineConfig`, `ExecutionBackend` | Attach resources and expose a native execution boundary. |
| `error` | `X86Error`, `Result` | Keep I/O, format, remote and backend failures typed. |

## `Image`

```rust
pub enum ImageKind {
    RawDisk,
    Iso9660,
    Bios,
    VgaBios,
    Kernel,
    Initrd,
    Bootloader,
    SavedState,
    Other,
}
```

Use `Image::from_bytes` for generated resources and `Image::from_file` for local files. `bytes`, `len`, `is_empty`, `kind`, `name` and `source` expose metadata without copying the image. `sha256` and `verify_sha256` provide deterministic integrity checks.

## `Resource` and `Bootloader`

```rust
pub enum Resource {
    File(std::path::PathBuf),
    Url(String),
    Bytes { name: String, bytes: Vec<u8> },
}
```

`Bootloader::load` uses default fetch settings. `Bootloader::load_with_options` accepts a timeout and user-agent. URL loading is compiled only when the `remote` feature is enabled; disabling default features makes the package suitable for strictly local deployments.

## `SavedState`

`SavedState::from_bytes` and `SavedState::from_file` validate the v86 magic, version, metadata length and required `state`/`buffer_infos` fields. The `summary` method returns encoded/decoded sizes, compression status, buffer count, memory size and SHA-256.

## `Machine`

`MachineConfig` uses builder methods such as `with_ram_bytes`, `with_vga_memory_bytes`, `with_cpu_hz`, `with_command_line` and `with_console`. The resource methods are:

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

`prepare` resets an attached backend. `run` accepts `RunOptions` and returns a `RunReport`. `stop` changes the lifecycle status to `Stopped`.

## `ExecutionBackend`

A backend is a native Rust object implementing:

```rust
pub trait ExecutionBackend: Send {
    fn reset(&mut self, config: &MachineConfig) -> Result<()>;
    fn step(&mut self) -> Result<bool>;
    fn read_memory(&self, address: u64, buffer: &mut [u8]) -> Result<()>;
    fn write_memory(&mut self, address: u64, data: &[u8]) -> Result<()>;
}
```

The boolean returned by `step` indicates that the backend has halted. This boundary keeps CPU/device implementation separate from the cross-platform resource and host API.

## Error handling

All fallible operations return `x86::Result<T>`. Errors distinguish filesystem failures, invalid image/state data, unsupported formats, remote failures, serialization failures and unavailable execution backends. Applications can match on `X86Error` rather than parsing strings.
