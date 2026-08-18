# Релизы

Релизы публикуются на странице [GitHub Releases](https://github.com/illussioon/x86/releases). Workflow репозитория собирает native artifacts для Linux, macOS Intel, macOS Apple Silicon и Windows при отправке version tag.

## Матрица targets

| Платформа | Target | Артефакты |
| --- | --- | --- |
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Консольный binary и `libx86.so`. |
| macOS Intel | `x86_64-apple-darwin` | Консольный binary и `libx86.dylib`. |
| macOS Apple Silicon | `aarch64-apple-darwin` | Консольный binary и `libx86.dylib`. |
| Windows x86_64 | `x86_64-pc-windows-msvc` | Консольный `.exe` и `x86.dll`. |

## Создание версии

```bash
git tag -a v0.1.0 -m "x86-native v0.1.0"
git push origin v0.1.0
```

GitHub Actions собирает каждый target, запускает тесты, формирует source package и прикладывает checksums. macOS binaries собираются на native macOS runners, а не линкуются в Linux sandbox: это обеспечивает корректные Apple SDK и linker.

## Локальная проверка релиза

```bash
cargo fmt -- --check
cargo check
cargo test
cargo package
cargo build --release
```

Имя package — `x86-native`, потому что `x86` уже занято в crates.io. Имя экспортируемой Rust library остаётся `x86`, поэтому код приложения выглядит так:

```rust
use x86::{Machine, MachineConfig};
```
