# x86-native v0.1.0

This is the first public native release of `x86-native`.

## Included

The release contains the cross-platform resource API, native bootloader/resource loading, v86 saved-state inspection, `Machine` lifecycle API, `ExecutionBackend` boundary, and the `x86-console` terminal application.

## Installation

```toml
[dependencies]
x86-native = "0.1"
```

The package exports the Rust library target `x86`.

## Platforms

Linux x86_64 and Windows x86_64 artifacts are attached directly. macOS Intel and Apple Silicon artifacts are built by the tag-triggered GitHub Actions workflow on native macOS runners and are attached when those jobs complete.

## Verification

The source package passed `cargo fmt --check`, `cargo check --all-targets`, `cargo test --all-targets` and `cargo package`. The release is not a complete x86 CPU/device emulator yet; `ExecutionBackend` remains the explicit extension boundary for the execution engine.
