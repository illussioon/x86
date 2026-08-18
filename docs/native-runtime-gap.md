# Native runtime gap analysis

## Current findings

The existing `x86-native` crate is a resource/state host API only. Its `ExecutionBackend` trait is not connected to a CPU or device implementation.

The original v86 repository contains a substantial Rust CPU core. After regenerating the missing instruction tables with the repository Makefile, the Rust core passes `cargo check --target x86_64-unknown-linux-gnu`. This confirms that the instruction interpreter and related CPU modules are portable Rust source, but it does not yet make the runtime usable.

## Blocking integration points

| Area | Current v86 dependency | Native work required |
| --- | --- | --- |
| CPU memory | Fixed WASM linear-memory offsets and `wasm32` pointer assumptions | Replace fixed offsets with an initialized native memory base and preserve the 32-bit guest address model. |
| Host callbacks | `io_port_*`, `mmap_*`, timers, exception hooks and random source are imported from JavaScript | Implement Rust host traits for I/O ports, MMIO, timers, IRQ delivery and deterministic/random source. |
| CPU execution | `main_loop` dispatches through WASM table/JIT or interpreter | Provide a native interpreter path first, then optionally native JIT; disable browser yielding. |
| State restore | `state.js` reconstructs nested arrays/maps/typed buffers and calls `CPU.set_state` | Parse v86 state JSON/buffers in Rust and map CPU indices, packed memory and device states. |
| Devices | State contains APIC, IOAPIC, PIC, PCI, PIT, RTC, VGA, PS/2, UART, IDE, virtio and 9p objects | Implement or port compatible native devices, especially virtio-9p for the Arch profile. |
| Console | Existing console only loads/inspects resources | Connect serial/VGA output and a real execution backend to `prepare`/`run`. |

## Important conclusion

`arch_state-v3.bin.zst` is a complete v86 saved state, not a raw disk. A correct native launch must restore the CPU plus the device graph represented in the state. Parsing the header or extracting buffers is not sufficient to boot Arch.

## Arch state layout observed

The supplied image contains a version 6 state with 512 MiB guest RAM, 86 top-level CPU state slots and 66 typed buffers. The CPU state includes register/segment/control buffers, packed guest memory at state slot 77 (54,517,760 bytes), a memory bitmap at slot 78 (16,384 bytes), APIC/PCI/RTC/VGA/PS/2/UART/IDE/PIT/network/virtio device arrays, and a 512-byte firmware value buffer. This confirms that restore must reconstruct both scalar CPU state and the packed-memory bitmap before the instruction loop can continue.
