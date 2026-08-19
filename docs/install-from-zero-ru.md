# Установка x86-native с нуля

Ниже приведены команды для запуска Arch Linux из `arch_state-v3.bin.zst` без браузера и WebAssembly. Для macOS Apple Silicon используется бинарник `aarch64`, для Intel Mac — `x86_64`.

## Вариант 1. macOS Apple Silicon — M1/M2/M3/M4

Откройте Terminal и выполните:

```sh
mkdir -p "$HOME/x86-native-macos-aarch64"
cd "$HOME/x86-native-macos-aarch64"

curl -fL --retry 3 -o x86-console-macos-aarch64 \
  https://github.com/illussioon/x86/releases/download/v0.1.7/x86-console-macos-aarch64

curl -fL --retry 3 -o arch_state-v3.bin.zst \
  https://i.copy.sh/arch_state-v3.bin.zst

chmod +x x86-console-macos-aarch64
xattr -d com.apple.quarantine x86-console-macos-aarch64 2>/dev/null || true
```

Проверьте архитектуру Mac:

```sh
uname -m
```

Для Apple Silicon результатом должен быть `arm64`. После этого запустите Arch прямо в терминале:

```sh
X86_TERM_COLS=80 X86_TERM_ROWS=24 \
printf 'load state ./arch_state-v3.bin.zst\nrun-state 1000000\n' \
  | ./x86-console-macos-aarch64
```

Для интерактивной работы запустите бинарник без pipe:

```sh
./x86-console-macos-aarch64
```

Внутри консоли выполните:

```text
load state ./arch_state-v3.bin.zst
run-state 1000000
type echo hello-from-arch
run-state 1000000
type ls
run-state 1000000
screen
quit
```

## Вариант 2. macOS Intel

```sh
mkdir -p "$HOME/x86-native-macos-x86_64"
cd "$HOME/x86-native-macos-x86_64"

curl -fL --retry 3 -o x86-console-macos-x86_64 \
  https://github.com/illussioon/x86/releases/download/v0.1.7/x86-console-macos-x86_64

curl -fL --retry 3 -o arch_state-v3.bin.zst \
  https://i.copy.sh/arch_state-v3.bin.zst

chmod +x x86-console-macos-x86_64
xattr -d com.apple.quarantine x86-console-macos-x86_64 2>/dev/null || true

uname -m
X86_TERM_COLS=80 X86_TERM_ROWS=24 \
printf 'load state ./arch_state-v3.bin.zst\nrun-state 1000000\n' \
  | ./x86-console-macos-x86_64
```

На Intel Mac `uname -m` обычно выводит `x86_64`.

## Вариант 3. Linux x86_64

```sh
mkdir -p "$HOME/x86-native-linux-x86_64"
cd "$HOME/x86-native-linux-x86_64"

curl -fL --retry 3 -o x86-console-linux-x86_64 \
  https://github.com/illussioon/x86/releases/download/v0.1.7/x86-console-linux-x86_64

curl -fL --retry 3 -o arch_state-v3.bin.zst \
  https://i.copy.sh/arch_state-v3.bin.zst

chmod +x x86-console-linux-x86_64

X86_TERM_COLS=100 X86_TERM_ROWS=32 \
printf 'load state ./arch_state-v3.bin.zst\nrun-state 1000000\n' \
  | ./x86-console-linux-x86_64
```

Интерактивный запуск:

```sh
./x86-console-linux-x86_64
```

Затем:

```text
load state ./arch_state-v3.bin.zst
run-state 1000000
type echo hello-from-linux-terminal
run-state 1000000
```

## Вариант 4. Windows x86_64 PowerShell

Откройте PowerShell и выполните:

```powershell
$dir = Join-Path $HOME "x86-native-windows-x86_64"
New-Item -ItemType Directory -Force $dir | Out-Null
Set-Location $dir

Invoke-WebRequest `
  -Uri "https://github.com/illussioon/x86/releases/download/v0.1.7/x86-console-windows-x86_64.exe" `
  -OutFile "x86-console-windows-x86_64.exe"

Invoke-WebRequest `
  -Uri "https://i.copy.sh/arch_state-v3.bin.zst" `
  -OutFile "arch_state-v3.bin.zst"

Unblock-File .\x86-console-windows-x86_64.exe
```

Запуск из PowerShell:

```powershell
$commands = @"
load state .\arch_state-v3.bin.zst
run-state 1000000
quit
"@

$commands | .\x86-console-windows-x86_64.exe
```

Интерактивный режим:

```powershell
.\x86-console-windows-x86_64.exe
```

После запуска используйте команды:

```text
load state .\arch_state-v3.bin.zst
run-state 1000000
type dir
run-state 1000000
screen
```

## Вариант 5. Установка через Cargo

Если Rust уже установлен, можно установить native console через crates.io:

```sh
cargo install x86-native --version 0.1.7 --locked
```

Проверьте установку:

```sh
x86-console --help
```

Скачайте saved state в отдельную папку:

```sh
mkdir -p "$HOME/x86-native-state"
cd "$HOME/x86-native-state"
curl -fL --retry 3 -o arch_state-v3.bin.zst \
  https://i.copy.sh/arch_state-v3.bin.zst
```

Запустите:

```sh
X86_TERM_COLS=80 X86_TERM_ROWS=24 \
printf 'load state ./arch_state-v3.bin.zst\nrun-state 1000000\n' \
  | x86-console
```

Для использования библиотеки в своём Rust-проекте:

```sh
cargo new my-x86-app
cd my-x86-app
cargo add x86-native@0.1.7
```

В `Cargo.toml` это соответствует:

```toml
[dependencies]
x86-native = "0.1.7"
```

## Размер экрана в терминале

Для graphical Arch state renderer использует Unicode half-blocks и ANSI 24-bit colors. По умолчанию используется размер 128×48 terminal cells. Для обычного окна macOS Terminal размер можно уменьшить:

```sh
X86_TERM_COLS=80 X86_TERM_ROWS=24
```

Для широкого терминала:

```sh
X86_TERM_COLS=120 X86_TERM_ROWS=40
```

Полная команда запуска должна выглядеть так:

```sh
X86_TERM_COLS=80 X86_TERM_ROWS=24 \
printf 'load state ./arch_state-v3.bin.zst\nrun-state 1000000\n' \
  | ./x86-console-macos-aarch64
```

## Проверка скачанных файлов

Проверить размер saved state:

```sh
ls -lh arch_state-v3.bin.zst
```

Проверить SHA-256 образа на macOS или Linux:

```sh
shasum -a 256 arch_state-v3.bin.zst
```

Для Windows PowerShell:

```powershell
Get-FileHash .\arch_state-v3.bin.zst -Algorithm SHA256
```

## Основные команды x86-console

| Команда | Назначение |
|---|---|
| `help` | Показать справку |
| `load state <path>` | Загрузить v86 saved state |
| `run-state <steps>` | Выполнить заданное число guest instructions и обновить экран |
| `screen` | Повторно вывести экран в текущий терминал |
| `type <text>` | Отправить строку и Enter в гостевую PS/2-клавиатуру |
| `dump-screen <path.ppm>` | Сохранить точный framebuffer в PPM |
| `info` | Показать конфигурацию и сведения о state |
| `checksum state` | Показать SHA-256 загруженного state |
| `quit` | Выйти |

## Ссылки

* [GitHub Release v0.1.7](https://github.com/illussioon/x86/releases/tag/v0.1.7)
* [GitHub repository](https://github.com/illussioon/x86)
* [crates.io: x86-native](https://crates.io/crates/x86-native)
* [Arch saved state](https://i.copy.sh/arch_state-v3.bin.zst)
