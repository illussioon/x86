# Releases

Releases are published on [GitHub Releases](https://github.com/illussioon/x86/releases). The repository workflow builds native artifacts for Linux, macOS Intel, macOS Apple Silicon and Windows when a version tag is pushed.

## Target matrix

| Platform | Target | Artifacts |
| --- | --- | --- |
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Console binary and `libx86.so`. |
| macOS Intel | `x86_64-apple-darwin` | Console binary and `libx86.dylib`. |
| macOS Apple Silicon | `aarch64-apple-darwin` | Console binary and `libx86.dylib`. |
| Windows x86_64 | `x86_64-pc-windows-msvc` | Console `.exe` and `x86.dll`. |

## Creating a version

```bash
git tag -a v0.1.0 -m "x86-native v0.1.0"
git push origin v0.1.0
```

The GitHub Actions workflow builds each target, runs tests, packages the source crate and attaches checksums. macOS binaries are built on native macOS runners rather than cross-linked on Linux, which provides the correct Apple SDK and linker.

## Local release checks

```bash
cargo fmt -- --check
cargo check
cargo test
cargo package
cargo build --release
```

The project keeps the package name `x86-native` because `x86` is already occupied in crates.io. The exported Rust library target is still `x86`, so downstream source code remains:

```rust
use x86::{Machine, MachineConfig};
```
