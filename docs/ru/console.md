# Нативная консоль

Запуск:

```bash
cargo run --bin x86-console
```

![Нативная консоль x86](../assets/console-ru.png)

Пример сессии:

```text
x86> capabilities
- raw-image-memory
- raw-image-file
- bootloader-file
- bootloader-http
- bootloader-https
- saved-state-v86-v6
- console-host-api
- backend-trait
- no-webassembly-runtime

x86> load state machine-state.bin.zst
saved state loaded
x86> load bootloader seabios.bin
bootloader loaded
x86> info
status: Created
saved-state: version 6, compressed=true, 66 buffers
x86> checksum state
state: <sha256>
x86> prepare
machine is not ready: no ExecutionBackend attached
x86> quit
```

Сообщение `no ExecutionBackend attached` ожидаемо: текущий host API отделяет подготовку ресурсов от фактического исполнения гостевой системы.
