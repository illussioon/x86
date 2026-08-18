# x86-native v0.1.1

Patch release focused on documentation, packaging and native release automation.

## Changes

* Added a multilingual README and documentation in English, Russian and Ukrainian.
* Added API, image-loading, console and architecture guides with examples and screenshots.
* Added CI for Linux, macOS and Windows.
* Added release automation for Linux x86_64, macOS Intel, macOS Apple Silicon and Windows x86_64.
* Migrated the Intel macOS runner from retired `macos-13` to `macos-15-intel`.
* Added `LICENSE` and `LICENSE.MIT` files and synchronized Cargo package metadata.

## Installation

```toml
[dependencies]
x86-native = "0.1.1"
```

The package exports the Rust library target `x86`.
