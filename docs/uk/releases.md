# Релізи

Релізи публікуються на сторінці [GitHub Releases](https://github.com/illussioon/x86/releases). Workflow репозиторію збирає native artifacts для Linux, macOS Intel, macOS Apple Silicon і Windows після push version tag.

## Матриця targets

| Платформа | Target | Артефакти |
| --- | --- | --- |
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Консольний binary і `libx86.so`. |
| macOS Intel | `x86_64-apple-darwin` | Консольний binary і `libx86.dylib`. |
| macOS Apple Silicon | `aarch64-apple-darwin` | Консольний binary і `libx86.dylib`. |
| Windows x86_64 | `x86_64-pc-windows-msvc` | Консольний `.exe` і `x86.dll`. |

## Створення версії

```bash
git tag -a v0.1.0 -m "x86-native v0.1.0"
git push origin v0.1.0
```

GitHub Actions збирає кожен target, запускає тести, формує source package і додає checksums. macOS binaries збираються на native macOS runners, а не лінкуються в Linux sandbox: це забезпечує правильні Apple SDK та linker.

## Локальна перевірка релізу

```bash
cargo fmt -- --check
cargo check
cargo test
cargo package
cargo build --release
```

Назва package — `x86-native`, оскільки `x86` вже зайнята в crates.io. Назва експортованої Rust library залишається `x86`, тому код застосунку виглядає так:

```rust
use x86::{Machine, MachineConfig};
```
