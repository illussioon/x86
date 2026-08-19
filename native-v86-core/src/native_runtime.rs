use crate::cpu::{apic, cpu, global_pointers, ioapic, memory, pic};
use crate::native_devices;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();
static UART0: OnceLock<Mutex<UartState>> = OnceLock::new();

#[derive(Default)]
struct UartState {
    ints: u8,
    baud_rate: u16,
    line_control: u8,
    lsr: u8,
    fifo_control: u8,
    ier: u8,
    iir: u8,
    modem_control: u8,
    modem_status: u8,
    scratch: u8,
    irq: u8,
    input: VecDeque<u8>,
}

fn uart0() -> &'static Mutex<UartState> {
    UART0.get_or_init(|| Mutex::new(UartState::default()))
}

fn uart_read(port: i32) -> i32 {
    let offset = (port - 0x3F8) as u8;
    let mut uart = uart0().lock().expect("UART0 mutex poisoned");
    match offset {
        0 if uart.line_control & 0x80 != 0 => (uart.baud_rate & 0xFF) as i32,
        0 => uart.input.pop_front().unwrap_or(0) as i32,
        1 if uart.line_control & 0x80 != 0 => (uart.baud_rate >> 8) as i32,
        1 => (uart.ier & 0x0F) as i32,
        2 => {
            let fifo = if uart.fifo_control & 1 != 0 { 0xC0 } else { 0 };
            (uart.iir | fifo) as i32
        }
        3 => uart.line_control as i32,
        4 => uart.modem_control as i32,
        5 => (uart.lsr | if uart.input.is_empty() { 0 } else { 0x01 }) as i32,
        6 => uart.modem_status as i32,
        7 => uart.scratch as i32,
        _ => 0xFF,
    }
}

fn restore_uart_state(state: &[serde_json::Value]) -> Result<(), String> {
    if state.len() < 11 {
        return Err(format!(
            "UART state has {} fields; expected 11",
            state.len()
        ));
    }
    let mut uart = uart0()
        .lock()
        .map_err(|_| "UART0 mutex poisoned".to_owned())?;
    uart.ints = state[0]
        .as_i64()
        .ok_or_else(|| "UART ints is not an integer".to_owned())? as u8;
    uart.baud_rate = state[1]
        .as_i64()
        .ok_or_else(|| "UART baud rate is not an integer".to_owned())? as u16;
    uart.line_control = state[2]
        .as_i64()
        .ok_or_else(|| "UART line control is not an integer".to_owned())?
        as u8;
    uart.lsr = state[3]
        .as_i64()
        .ok_or_else(|| "UART LSR is not an integer".to_owned())? as u8;
    uart.fifo_control = state[4]
        .as_i64()
        .ok_or_else(|| "UART FIFO control is not an integer".to_owned())?
        as u8;
    uart.ier = state[5]
        .as_i64()
        .ok_or_else(|| "UART IER is not an integer".to_owned())? as u8;
    uart.iir = state[6]
        .as_i64()
        .ok_or_else(|| "UART IIR is not an integer".to_owned())? as u8;
    uart.modem_control = state[7]
        .as_i64()
        .ok_or_else(|| "UART modem control is not an integer".to_owned())?
        as u8;
    uart.modem_status = state[8]
        .as_i64()
        .ok_or_else(|| "UART modem status is not an integer".to_owned())?
        as u8;
    uart.scratch = state[9]
        .as_i64()
        .ok_or_else(|| "UART scratch is not an integer".to_owned())? as u8;
    uart.irq = state[10]
        .as_i64()
        .ok_or_else(|| "UART IRQ is not an integer".to_owned())? as u8;
    Ok(())
}

fn uart_write(port: i32, value: i32) {
    let offset = (port - 0x3F8) as u8;
    let byte = value as u8;
    let mut output = None;
    {
        let mut uart = uart0().lock().expect("UART0 mutex poisoned");
        match offset {
            0 if uart.line_control & 0x80 != 0 => {
                uart.baud_rate = (uart.baud_rate & 0xFF00) | byte as u16;
            }
            0 => output = Some(byte),
            1 if uart.line_control & 0x80 != 0 => {
                uart.baud_rate = (uart.baud_rate & 0x00FF) | ((byte as u16) << 8);
            }
            1 => uart.ier = byte & 0x0F,
            2 => uart.fifo_control = byte,
            3 => uart.line_control = byte,
            4 => uart.modem_control = byte,
            7 => uart.scratch = byte,
            _ => {}
        }
    }
    if let Some(byte) = output {
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(&[byte]);
        let _ = stdout.flush();
    }
}

/// Minimal native host callbacks used by the v86 CPU core.
/// Device-specific MMIO/port routing is intentionally represented as a small
/// host surface first; concrete PC devices are added by the outer runtime.
#[no_mangle]
pub extern "C" fn cpu_exception_hook(_interrupt: i32) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn microtick() -> f64 {
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

#[no_mangle]
pub extern "C" fn run_hardware_timers(_acpi_enabled: bool, _now: f64) -> f64 {
    0.0
}

#[no_mangle]
pub extern "C" fn cpu_event_halt() {}

#[no_mangle]
pub extern "C" fn stop_idling() {}

#[no_mangle]
pub extern "C" fn get_rand_int() -> i32 {
    0x1357_9BDF
}

#[no_mangle]
pub extern "C" fn io_port_read8(port: i32) -> i32 {
    if let Some(value) = native_devices::io_read8(port) {
        value
    } else if (0x3F8..=0x3FF).contains(&port) {
        uart_read(port)
    } else {
        0xFF
    }
}

#[no_mangle]
pub extern "C" fn io_port_read16(port: i32) -> i32 {
    native_devices::io_read32(port).unwrap_or(0xFFFF)
}

#[no_mangle]
pub extern "C" fn io_port_read32(port: i32) -> i32 {
    native_devices::io_read32(port).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn io_port_write8(port: i32, value: i32) {
    if !native_devices::io_write8(port, value) && (0x3F8..=0x3FF).contains(&port) {
        uart_write(port, value);
    }
}

#[no_mangle]
pub extern "C" fn io_port_write16(port: i32, value: i32) {
    if !native_devices::io_write16(port, value) {}
}

#[no_mangle]
pub extern "C" fn io_port_write32(port: i32, value: i32) {
    if !native_devices::io_write32(port, value) {}
}

#[no_mangle]
pub extern "C" fn mmap_read8(_addr: u32) -> i32 {
    0xFF
}

#[no_mangle]
pub extern "C" fn mmap_read32(_addr: u32) -> i32 {
    -1
}

#[no_mangle]
pub extern "C" fn mmap_write8(_addr: u32, _value: i32) {}

#[no_mangle]
pub extern "C" fn mmap_write16(_addr: u32, _value: i32) {}

#[no_mangle]
pub extern "C" fn mmap_write32(_addr: u32, _value: i32) {}

#[no_mangle]
pub extern "C" fn mmap_write64(_addr: u32, _v0: i32, _v1: i32) {}

#[no_mangle]
pub extern "C" fn mmap_write128(_addr: u32, _v0: i32, _v1: i32, _v2: i32, _v3: i32) {}

/// Native CPU state arena and guest memory owner.
///
/// v86's scalar CPU state uses the first 4 KiB of the arena. The guest RAM is
/// allocated by the core memory module and addressed with 32-bit guest physical
/// addresses, matching the original emulator model.
pub struct NativeCpu {
    state_arena: Box<[u8; 4096]>,
    ram_bytes: u32,
    vga_bytes: u32,
    last_timer_tick: Instant,
}

impl NativeCpu {
    pub fn new(ram_bytes: u32, vga_bytes: u32) -> Self {
        assert!(ram_bytes > 0, "RAM size must be non-zero");
        assert!(vga_bytes > 0, "VGA memory size must be non-zero");

        let mut state_arena = Box::new([0u8; 4096]);
        unsafe {
            global_pointers::init(state_arena.as_mut_ptr());
            let _ = memory::allocate_memory(ram_bytes);
            let _ = memory::svga_allocate_memory(vga_bytes);
            *global_pointers::memory_size = ram_bytes;
            memory::vga_memory_size = vga_bytes;
            cpu::reset_cpu();
        }

        Self {
            state_arena,
            ram_bytes,
            vga_bytes,
            last_timer_tick: Instant::now(),
        }
    }

    pub fn ram_bytes(&self) -> u32 {
        self.ram_bytes
    }

    pub fn vga_bytes(&self) -> u32 {
        self.vga_bytes
    }

    pub fn step(&mut self, max_instructions: u32) -> u32 {
        unsafe {
            let halted = *global_pointers::in_hlt;
            let timer_due = self.last_timer_tick.elapsed() >= std::time::Duration::from_millis(1);
            if halted || timer_due {
                let now = microtick();
                if *global_pointers::acpi_enabled {
                    let _ = apic::apic_timer(now);
                    cpu::handle_irqs();
                } else {
                    pic::set_irq(0);
                    cpu::handle_irqs();
                    pic::clear_irq(0);
                    cpu::handle_irqs();
                }
                self.last_timer_tick = Instant::now();
            }
            cpu::main_loop_native_interpreter(max_instructions)
        }
    }

    pub fn read_memory(&self, address: u32, output: &mut [u8]) -> bool {
        if address.checked_add(output.len() as u32).is_none()
            || address + output.len() as u32 > self.ram_bytes
        {
            return false;
        }
        unsafe {
            output.copy_from_slice(std::slice::from_raw_parts(
                memory::mem8.add(address as usize),
                output.len(),
            ));
        }
        true
    }

    pub fn write_memory(&mut self, address: u32, input: &[u8]) -> bool {
        if address.checked_add(input.len() as u32).is_none()
            || address + input.len() as u32 > self.ram_bytes
        {
            return false;
        }
        unsafe {
            std::slice::from_raw_parts_mut(memory::mem8.add(address as usize), input.len())
                .copy_from_slice(input);
        }
        true
    }

    pub fn instruction_pointer(&self) -> u32 {
        unsafe { *global_pointers::instruction_pointer as u32 }
    }

    pub fn halted(&self) -> bool {
        unsafe { *global_pointers::in_hlt }
    }

    pub fn state_arena(&self) -> &[u8; 4096] {
        &self.state_arena
    }

    pub fn set_9p_root(&mut self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        native_devices::set_9p_root(path)
    }
}

#[cfg(test)]
mod tests {
    use super::NativeCpu;

    #[test]
    fn native_interpreter_executes_reset_vector_hlt() {
        let mut cpu = NativeCpu::new(128 * 1024 * 1024, 8 * 1024 * 1024);
        assert!(cpu.write_memory(0xFFFF0, &[0xF4]));
        assert_eq!(cpu.instruction_pointer(), 0xFFFF0);
        assert_eq!(cpu.step(1), 1);
        assert!(cpu.halted());
    }
}

impl NativeCpu {
    /// Restore the CPU scalar state and packed RAM representation from the
    /// decoded v86 state object. Device arrays are intentionally left to the
    /// outer native device graph, but CPU execution can continue after this
    /// method completes.
    pub fn restore_v86_state(
        &mut self,
        state: &serde_json::Value,
        buffers: &[Vec<u8>],
    ) -> Result<(), String> {
        let slots = state
            .as_array()
            .ok_or_else(|| "v86 state is not an array".to_owned())?;

        let memory_size = scalar(slots, 0)? as u32;
        if memory_size != self.ram_bytes {
            return Err(format!(
                "state RAM is {memory_size} bytes, NativeCpu has {} bytes",
                self.ram_bytes
            ));
        }

        let segment_state = buffer_for(slots, buffers, 1)?;
        if segment_state.len() != 16 {
            return Err(format!(
                "state[1] length {} != expected 16",
                segment_state.len()
            ));
        }
        unsafe {
            std::slice::from_raw_parts_mut(global_pointers::segment_is_null as *mut u8, 8)
                .copy_from_slice(&segment_state[..8]);
            std::slice::from_raw_parts_mut(global_pointers::segment_access_bytes, 8)
                .copy_from_slice(&segment_state[8..]);
        }
        copy_i32_buffer(slots, buffers, 2, unsafe {
            std::slice::from_raw_parts_mut(global_pointers::segment_offsets as *mut u8, 32)
        })?;
        copy_u32_buffer(slots, buffers, 3, unsafe {
            std::slice::from_raw_parts_mut(global_pointers::segment_limits as *mut u8, 32)
        })?;

        unsafe {
            *global_pointers::memory_size = memory_size;
            *global_pointers::protected_mode = scalar(slots, 4)? != 0;
            *global_pointers::idtr_offset = scalar(slots, 5)? as i32;
            *global_pointers::idtr_size = scalar(slots, 6)? as i32;
            *global_pointers::gdtr_offset = scalar(slots, 7)? as i32;
            *global_pointers::gdtr_size = scalar(slots, 8)? as i32;
        }
        copy_i32_buffer(slots, buffers, 10, unsafe {
            std::slice::from_raw_parts_mut(global_pointers::cr as *mut u8, 32)
        })?;
        unsafe {
            *global_pointers::cpl = scalar(slots, 11)? as u8;
            *global_pointers::is_32 = scalar(slots, 13)? != 0;
            *global_pointers::stack_size_32 = scalar(slots, 16)? != 0;
            *global_pointers::in_hlt = scalar(slots, 17)? != 0;
            *global_pointers::last_virt_eip = scalar(slots, 18)? as i32;
            *global_pointers::eip_phys = scalar(slots, 19)? as i32;
            *global_pointers::sysenter_cs = scalar(slots, 22)? as i32;
            *global_pointers::sysenter_eip = scalar(slots, 23)? as i32;
            *global_pointers::sysenter_esp = scalar(slots, 24)? as i32;
            *global_pointers::prefixes = scalar(slots, 25)? as u8;
            *global_pointers::flags = scalar(slots, 26)? as i32;
            *global_pointers::flags_changed = scalar(slots, 27)? as i32;
            *global_pointers::last_op1 = scalar(slots, 28)? as i32;
            *global_pointers::last_op_size = scalar(slots, 30)? as i32;
            *global_pointers::instruction_pointer = scalar(slots, 37)? as i32;
            *global_pointers::previous_ip = scalar(slots, 38)? as i32;
        }
        copy_i32_buffer(slots, buffers, 39, unsafe {
            std::slice::from_raw_parts_mut(global_pointers::reg32 as *mut u8, 32)
        })?;
        copy_u16_buffer(slots, buffers, 40, unsafe {
            std::slice::from_raw_parts_mut(global_pointers::sreg as *mut u8, 16)
        })?;
        copy_i32_buffer(slots, buffers, 41, unsafe {
            std::slice::from_raw_parts_mut(global_pointers::dreg as *mut u8, 32)
        })?;
        copy_u64_buffer(slots, buffers, 42, unsafe {
            std::slice::from_raw_parts_mut(global_pointers::reg_pdpte as *mut u8, 32)
        })?;

        let tsc = buffer_for(slots, buffers, 43)?;
        if tsc.len() >= 8 {
            let low = u32::from_le_bytes(tsc[0..4].try_into().unwrap());
            let high = u32::from_le_bytes(tsc[4..8].try_into().unwrap());
            unsafe {
                cpu::set_tsc(low, high);
            }
        }

        if let Some(uart_state) = slots.get(54).and_then(serde_json::Value::as_array) {
            restore_uart_state(uart_state)?;
        }
        if let Some(pic_state) = slots.get(60).and_then(serde_json::Value::as_array) {
            let master = byte_array_from_state(pic_state, 13, "PIC master")?;
            let slave_value = pic_state
                .get(5)
                .ok_or_else(|| "PIC state has no slave controller".to_owned())?;
            let slave_array = slave_value
                .as_array()
                .ok_or_else(|| "PIC slave state is not an array".to_owned())?;
            let slave = byte_array_from_values(slave_array, 13, "PIC slave")?;
            pic::restore_state(&master, &slave);
        }

        if slots.get(46).is_some_and(|value| !value.is_null()) {
            let apic_state = buffer_for(slots, buffers, 46)?;
            apic::restore_state_bytes(apic_state)?;
            unsafe {
                *global_pointers::apic_enabled = true;
                *global_pointers::acpi_enabled = true;
            }
        }
        if slots.get(63).is_some_and(|value| !value.is_null()) {
            let ioapic_state = buffer_for(slots, buffers, 63)?;
            ioapic::restore_state_bytes(ioapic_state)?;
        }

        unsafe {
            *global_pointers::tss_size_32 = scalar(slots, 64)? != 0;
        }
        copy_buffer(slots, buffers, 66, unsafe {
            std::slice::from_raw_parts_mut(global_pointers::reg_xmm as *mut u8, 128)
        })?;
        copy_buffer(slots, buffers, 67, unsafe {
            std::slice::from_raw_parts_mut(global_pointers::fpu_st as *mut u8, 128)
        })?;
        unsafe {
            *global_pointers::fpu_stack_empty = scalar(slots, 68)? as u8;
            *global_pointers::fpu_stack_ptr = scalar(slots, 69)? as u8;
            *global_pointers::fpu_control_word = scalar(slots, 70)? as u16;
            *global_pointers::fpu_ip = scalar(slots, 71)? as i32;
            *global_pointers::fpu_ip_selector = scalar(slots, 72)? as i32;
            *global_pointers::fpu_dp = scalar(slots, 73)? as i32;
            *global_pointers::fpu_dp_selector = scalar(slots, 74)? as i32;
            *global_pointers::fpu_opcode = scalar(slots, 75)? as i32;
            *global_pointers::last_result = slots
                .get(86)
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as i32;
            *global_pointers::fpu_status_word = slots
                .get(87)
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as u16;
            *global_pointers::mxcsr = slots
                .get(88)
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0x1F80) as i32;
        }

        let packed_memory = buffer_for(slots, buffers, 77)?;
        let bitmap = buffer_for(slots, buffers, 78)?;
        unsafe {
            std::ptr::write_bytes(memory::mem8, 0, self.ram_bytes as usize);
        }
        let page_count = self.ram_bytes as usize / 0x1000;
        let mut packed_page = 0usize;
        for page in 0..page_count {
            if bitmap
                .get(page >> 3)
                .map_or(false, |byte| byte & (1 << (page & 7)) != 0)
            {
                let src_start = packed_page * 0x1000;
                let src_end = src_start + 0x1000;
                if src_end > packed_memory.len() {
                    return Err("packed memory buffer is shorter than bitmap population".to_owned());
                }
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        packed_memory.as_ptr().add(src_start),
                        memory::mem8.add(page * 0x1000),
                        0x1000,
                    );
                }
                packed_page += 1;
            }
        }
        if packed_page * 0x1000 != packed_memory.len() {
            return Err(format!(
                "packed memory has {} pages but bitmap references {}",
                packed_memory.len() / 0x1000,
                packed_page
            ));
        }

        native_devices::restore_state(state, buffers)?;
        cpu::update_state_flags();
        unsafe {
            cpu::full_clear_tlb();
        }
        Ok(())
    }
}

fn buffer_for<'a>(
    state: &[serde_json::Value],
    buffers: &'a [Vec<u8>],
    index: usize,
) -> Result<&'a [u8], String> {
    let buffer_id = state
        .get(index)
        .and_then(serde_json::Value::as_object)
        .and_then(|object| object.get("buffer_id"))
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("state[{index}] is not a typed buffer"))?
        as usize;
    buffers
        .get(buffer_id)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("buffer id {buffer_id} is out of range"))
}

fn byte_array_from_state(
    state: &[serde_json::Value],
    len: usize,
    name: &str,
) -> Result<[u8; 13], String> {
    byte_array_from_values(state, len, name)
}

fn byte_array_from_values(
    state: &[serde_json::Value],
    len: usize,
    name: &str,
) -> Result<[u8; 13], String> {
    if len != 13 || state.len() < len {
        return Err(format!("{name} has {} fields; expected {len}", state.len()));
    }
    let mut result = [0u8; 13];
    for (index, value) in state.iter().take(len).enumerate() {
        if index == 5 {
            // v86 stores the slave PIC array at master[5]; Pic0 byte five is
            // only a legacy dummy slot and is not part of the nested state.
            continue;
        }
        result[index] = value
            .as_i64()
            .ok_or_else(|| format!("{name}[{index}] is not an integer"))?
            as u8;
    }
    Ok(result)
}

fn scalar(state: &[serde_json::Value], index: usize) -> Result<i64, String> {
    state
        .get(index)
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| format!("state[{index}] is not an integer scalar"))
}

fn copy_buffer(
    state: &[serde_json::Value],
    buffers: &[Vec<u8>],
    index: usize,
    target: &mut [u8],
) -> Result<(), String> {
    let source = buffer_for(state, buffers, index)?;
    if source.len() != target.len() {
        return Err(format!(
            "state[{index}] length {} != expected {}",
            source.len(),
            target.len()
        ));
    }
    target.copy_from_slice(source);
    Ok(())
}

fn copy_i32_buffer(
    state: &[serde_json::Value],
    buffers: &[Vec<u8>],
    index: usize,
    target: &mut [u8],
) -> Result<(), String> {
    copy_buffer(state, buffers, index, target)
}

fn copy_u16_buffer(
    state: &[serde_json::Value],
    buffers: &[Vec<u8>],
    index: usize,
    target: &mut [u8],
) -> Result<(), String> {
    copy_buffer(state, buffers, index, target)
}

fn copy_u32_buffer(
    state: &[serde_json::Value],
    buffers: &[Vec<u8>],
    index: usize,
    target: &mut [u8],
) -> Result<(), String> {
    copy_buffer(state, buffers, index, target)
}

fn copy_u64_buffer(
    state: &[serde_json::Value],
    buffers: &[Vec<u8>],
    index: usize,
    target: &mut [u8],
) -> Result<(), String> {
    copy_buffer(state, buffers, index, target)
}
