# Terminal renderer implementation findings

The native runtime exposes mmap callbacks in `native_runtime.rs` at lines 640–701. Legacy memory callbacks currently treat the entire `0xA0000..0xC0000` range as one buffer and index it as `addr - 0xA0000`. The saved Arch state buffer 35 is 262144 bytes and begins with VGA text-like cells (`53 07 65 07 ...`, i.e. `S`, `e`, ...), so the terminal snapshot should read the restored buffer directly rather than guest RAM.

The native core now has a process-global `VGA_TEXT: OnceLock<Mutex<Vec<u8>>>` containing 0x40000 bytes, a restore hook for VGA state slot 6/buffer 35, and `NativeCpu::vga_text_snapshot()` returning 80x25 char/attribute bytes. The mapping still needs to be corrected so guest address `0xB8000` maps to the beginning of this saved-state text buffer; this is the next patch. The public `ExecutionBackend`, `Machine`, and `NativeBackend` text snapshot methods have already been added.
