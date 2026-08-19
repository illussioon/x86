use crate::cpu::{apic, cpu, global_pointers, ioapic, memory, pic};
use crate::native_devices;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();
static UART0: OnceLock<Mutex<UartState>> = OnceLock::new();
static PS2: OnceLock<Mutex<Ps2State>> = OnceLock::new();
static PIT: OnceLock<Mutex<PitState>> = OnceLock::new();
static RTC: OnceLock<Mutex<RtcState>> = OnceLock::new();
static VGA_TEXT: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();
static UART_OUTPUT_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn set_uart_output_enabled(enabled: bool) {
    UART_OUTPUT_ENABLED.store(enabled, Ordering::Relaxed);
}

fn vga_text_memory() -> &'static Mutex<Vec<u8>> {
    VGA_TEXT.get_or_init(|| Mutex::new(vec![0; 0x40000]))
}

fn legacy_vga_offset(addr: u32) -> Option<usize> {
    // Preserve the conventional A0000 graphics window. The saved v86 VGA
    // state stores the text plane at logical offset zero, so B8000 is also
    // exposed as an alias to offset zero for the terminal snapshot.
    if (0xA0000..0xB8000).contains(&addr) {
        Some((addr - 0xA0000) as usize)
    } else if (0xB8000..0xC0000).contains(&addr) {
        Some((addr - 0xB8000) as usize)
    } else {
        None
    }
}

#[derive(Clone)]
struct RtcState {
    index: u8,
    data: [u8; 128],
    status_a: u8,
    status_b: u8,
    status_c: u8,
    status_d: u8,
    nmi_disabled: bool,
}

impl Default for RtcState {
    fn default() -> Self {
        let mut data = [0u8; 128];
        data[0x0A] = 0x26;
        data[0x0B] = 0x02;
        data[0x0D] = 0x80;
        Self {
            index: 0,
            data,
            status_a: 0x26,
            status_b: 0x02,
            status_c: 0,
            status_d: 0x80,
            nmi_disabled: false,
        }
    }
}

fn rtc() -> &'static Mutex<RtcState> {
    RTC.get_or_init(|| Mutex::new(RtcState::default()))
}

fn rtc_read(port: i32) -> Option<i32> {
    let state = rtc().lock().ok()?;
    match port {
        0x71 => Some(match state.index & 0x7F {
            0x0A => state.status_a,
            0x0B => state.status_b,
            0x0C => state.status_c,
            0x0D => state.status_d,
            index => state.data[index as usize],
        } as i32),
        0x70 => Some(state.index as i32 | if state.nmi_disabled { 0x80 } else { 0 }),
        _ => None,
    }
}

fn rtc_write(port: i32, value: i32) -> bool {
    let Ok(mut state) = rtc().lock() else {
        return false;
    };
    match port {
        0x70 => {
            let byte = value as u8;
            state.index = byte & 0x7F;
            state.nmi_disabled = byte & 0x80 != 0;
            true
        }
        0x71 => {
            let index = state.index & 0x7F;
            let byte = value as u8;
            match index {
                0x0A => state.status_a = byte,
                0x0B => state.status_b = byte,
                0x0C | 0x0D => {}
                _ => state.data[index as usize] = byte,
            }
            true
        }
        _ => false,
    }
}

#[derive(Clone)]
struct PitState {
    next_low: [u8; 3],
    enabled: [bool; 3],
    mode: [u8; 3],
    read_mode: [u8; 3],
    latch: [u8; 3],
    latch_value: [u16; 3],
    reload: [u16; 3],
    start_value: [u16; 3],
    start: [Instant; 3],
}

impl Default for PitState {
    fn default() -> Self {
        Self {
            next_low: [1; 3],
            enabled: [false; 3],
            mode: [3; 3],
            read_mode: [3; 3],
            latch: [0; 3],
            latch_value: [0; 3],
            reload: [0; 3],
            start_value: [0; 3],
            start: [Instant::now(), Instant::now(), Instant::now()],
        }
    }
}

fn pit() -> &'static Mutex<PitState> {
    PIT.get_or_init(|| Mutex::new(PitState::default()))
}

const PIT_HZ: f64 = 1_193_181.6666;

fn pit_counter_value(state: &PitState, channel: usize) -> u16 {
    if !state.enabled[channel] || state.reload[channel] == 0 {
        return 0;
    }
    let elapsed = state.start[channel].elapsed().as_secs_f64();
    let ticks = (elapsed * PIT_HZ) as u64;
    let reload = state.reload[channel] as u64;
    (state.start_value[channel] as u64).wrapping_sub(ticks % reload.max(1)) as u16
}

fn pit_read(port: i32) -> Option<i32> {
    if !(0x40..=0x42).contains(&port) {
        return None;
    }
    let channel = (port - 0x40) as usize;
    let mut state = pit().lock().ok()?;
    if state.latch[channel] != 0 {
        state.latch[channel] -= 1;
        return Some(if state.latch[channel] == 1 {
            (state.latch_value[channel] & 0xFF) as i32
        } else {
            (state.latch_value[channel] >> 8) as i32
        });
    }
    let value = pit_counter_value(&state, channel);
    let low = state.next_low[channel] != 0;
    if state.mode[channel] == 3 {
        state.next_low[channel] ^= 1;
    }
    Some(if low {
        (value & 0xFF) as i32
    } else {
        (value >> 8) as i32
    })
}

fn pit_poll() -> bool {
    let Ok(mut state) = pit().lock() else {
        return false;
    };
    if !state.enabled[0] || state.reload[0] == 0 {
        return false;
    }
    let elapsed_ticks = (state.start[0].elapsed().as_secs_f64() * PIT_HZ) as u64;
    if elapsed_ticks >= state.start_value[0] as u64 {
        state.start[0] = Instant::now();
        state.start_value[0] = state.reload[0];
        drop(state);
        unsafe {
            crate::cpu::cpu::device_lower_irq(0);
            crate::cpu::cpu::device_raise_irq(0);
        }
    }
    true
}

fn pit_write(port: i32, value: i32) -> bool {
    let Ok(mut state) = pit().lock() else {
        return false;
    };
    if (0x40..=0x42).contains(&port) {
        let channel = (port - 0x40) as usize;
        let byte = value as u8;
        if state.next_low[channel] != 0 {
            state.reload[channel] = (state.reload[channel] & 0xFF00) | byte as u16;
        } else {
            state.reload[channel] = (state.reload[channel] & 0x00FF) | ((byte as u16) << 8);
            if state.reload[channel] == 0 {
                state.reload[channel] = 0xFFFF;
            }
            state.start_value[channel] = state.reload[channel];
            state.start[channel] = Instant::now();
            state.enabled[channel] = true;
        }
        state.next_low[channel] ^= 1;
        return true;
    }
    if port == 0x43 {
        let command = value as u8;
        let channel = ((command >> 6) & 3) as usize;
        if channel >= 3 {
            return true;
        }
        let read_mode = (command >> 4) & 3;
        if read_mode == 0 {
            state.latch_value[channel] = pit_counter_value(&state, channel);
            state.latch[channel] = 2;
        } else {
            state.read_mode[channel] = read_mode;
            state.mode[channel] = (command >> 1) & 7;
            state.next_low[channel] = if read_mode == 3 { 1 } else { 0 };
        }
        return true;
    }
    port == 0x61
}

#[derive(Default)]
struct Ps2State {
    output: VecDeque<u8>,
    command_byte: u8,
    pending_command: u8,
}

fn ps2() -> &'static Mutex<Ps2State> {
    PS2.get_or_init(|| {
        Mutex::new(Ps2State {
            command_byte: 0x01,
            ..Ps2State::default()
        })
    })
}

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

fn ps2_read(port: i32) -> Option<i32> {
    let mut controller = ps2().lock().ok()?;
    match port {
        0x60 => {
            let value = controller.output.pop_front().unwrap_or(0);
            let more = !controller.output.is_empty();
            drop(controller);
            unsafe {
                crate::cpu::cpu::device_lower_irq(1);
                if more {
                    crate::cpu::cpu::device_raise_irq(1);
                }
            }
            Some(value as i32)
        }
        0x64 => Some(if controller.output.is_empty() { 0 } else { 1 }),
        _ => None,
    }
}

fn ps2_write(port: i32, value: i32) -> bool {
    let Ok(mut controller) = ps2().lock() else {
        return false;
    };
    match port {
        0x64 => {
            controller.pending_command = value as u8;
            true
        }
        0x60 => {
            if controller.pending_command == 0x60 {
                controller.command_byte = value as u8;
            }
            controller.pending_command = 0;
            true
        }
        _ => false,
    }
}

fn keycode_for_ascii(byte: u8) -> Option<(u8, bool)> {
    let upper = byte.to_ascii_uppercase();
    let shifted = byte.is_ascii_uppercase();
    let code = match upper {
        b'A' => 0x1E,
        b'B' => 0x30,
        b'C' => 0x2E,
        b'D' => 0x20,
        b'E' => 0x12,
        b'F' => 0x21,
        b'G' => 0x22,
        b'H' => 0x23,
        b'I' => 0x17,
        b'J' => 0x24,
        b'K' => 0x25,
        b'L' => 0x26,
        b'M' => 0x32,
        b'N' => 0x31,
        b'O' => 0x18,
        b'P' => 0x19,
        b'Q' => 0x10,
        b'R' => 0x13,
        b'S' => 0x1F,
        b'T' => 0x14,
        b'U' => 0x16,
        b'V' => 0x2F,
        b'W' => 0x11,
        b'X' => 0x2D,
        b'Y' => 0x15,
        b'Z' => 0x2C,
        b'1' | b'!' => 0x02,
        b'2' | b'@' => 0x03,
        b'3' | b'#' => 0x04,
        b'4' | b'$' => 0x05,
        b'5' | b'%' => 0x06,
        b'6' | b'^' => 0x07,
        b'7' | b'&' => 0x08,
        b'8' | b'*' => 0x09,
        b'9' | b'(' => 0x0A,
        b'0' | b')' => 0x0B,
        b'-' | b'_' => 0x0C,
        b'=' | b'+' => 0x0D,
        b'[' | b'{' => 0x1A,
        b']' | b'}' => 0x1B,
        b';' | b':' => 0x27,
        b'\'' | b'"' => 0x28,
        b'`' | b'~' => 0x29,
        b'\\' | b'|' => 0x2B,
        b',' | b'<' => 0x33,
        b'.' | b'>' => 0x34,
        b'/' | b'?' => 0x35,
        b' ' => 0x39,
        b'\n' | b'\r' => 0x1C,
        b'\t' => 0x0F,
        8 => 0x0E,
        _ => return None,
    };
    let shifted = shifted
        || matches!(byte, b'!'..=b'&' | b'('..=b'+' | b':' | b'<'..=b'>' | b'?' | b'@' | b'^' | b'_' | b'{' | b'|' | b'}' | b'~' | b'"');
    Some((code, shifted))
}

pub fn inject_keyboard_text(text: &str) -> usize {
    let Ok(mut controller) = ps2().lock() else {
        return 0;
    };
    let mut count = 0;
    for byte in text.bytes() {
        let Some((code, shifted)) = keycode_for_ascii(byte) else {
            continue;
        };
        if shifted {
            controller.output.push_back(0x2A);
        }
        controller.output.push_back(code);
        controller.output.push_back(code | 0x80);
        if shifted {
            controller.output.push_back(0xAA);
        }
        count += 1;
    }
    drop(controller);
    if count > 0 {
        unsafe { crate::cpu::cpu::device_raise_irq(1) };
    }
    count
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

fn nested_buffer<'a>(
    state: &[serde_json::Value],
    index: usize,
    buffers: &'a [Vec<u8>],
) -> Result<&'a [u8], String> {
    let buffer_id = state
        .get(index)
        .and_then(serde_json::Value::as_object)
        .and_then(|object| object.get("buffer_id"))
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("nested state[{index}] is not a typed buffer"))?
        as usize;
    buffers
        .get(buffer_id)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("nested buffer id {buffer_id} is out of range"))
}

fn restore_rtc_state(state: &[serde_json::Value], buffers: &[Vec<u8>]) -> Result<(), String> {
    if state.len() < 14 {
        return Err(format!("RTC state has {} fields; expected 14", state.len()));
    }
    let data = nested_buffer(state, 1, buffers)?;
    let mut rtc = rtc().lock().map_err(|_| "RTC mutex poisoned".to_owned())?;
    rtc.index = state[0].as_i64().unwrap_or(0) as u8;
    rtc.data.fill(0);
    let copy_len = data.len().min(rtc.data.len());
    rtc.data[..copy_len].copy_from_slice(&data[..copy_len]);
    rtc.status_a = state[8].as_i64().unwrap_or(rtc.data[0x0A] as i64) as u8;
    rtc.status_b = state[9].as_i64().unwrap_or(rtc.data[0x0B] as i64) as u8;
    rtc.status_c = state[10].as_i64().unwrap_or(0) as u8;
    rtc.nmi_disabled = state[11].as_i64().unwrap_or(0) != 0;
    rtc.status_d = rtc.data[0x0D].max(0x80);
    Ok(())
}

fn restore_pit_state(state: &[serde_json::Value], buffers: &[Vec<u8>]) -> Result<(), String> {
    if state.len() < 9 {
        return Err(format!("PIT state has {} fields; expected 9", state.len()));
    }
    let next_low = nested_buffer(state, 0, buffers)?;
    let enabled = nested_buffer(state, 1, buffers)?;
    let mode = nested_buffer(state, 2, buffers)?;
    let read_mode = nested_buffer(state, 3, buffers)?;
    let latch = nested_buffer(state, 4, buffers)?;
    let reload = nested_buffer(state, 6, buffers)?;
    let start_value = nested_buffer(state, 8, buffers)?;
    let mut pit = pit().lock().map_err(|_| "PIT mutex poisoned".to_owned())?;
    for channel in 0..3 {
        pit.next_low[channel] = *next_low.get(channel).unwrap_or(&1);
        pit.enabled[channel] = *enabled.get(channel).unwrap_or(&0) != 0;
        pit.mode[channel] = *mode.get(channel).unwrap_or(&3);
        pit.read_mode[channel] = *read_mode.get(channel).unwrap_or(&3);
        pit.latch[channel] = *latch.get(channel).unwrap_or(&0);
        let offset = channel * 2;
        pit.reload[channel] = u16::from_le_bytes([
            *reload.get(offset).unwrap_or(&0),
            *reload.get(offset + 1).unwrap_or(&0),
        ]);
        pit.start_value[channel] = u16::from_le_bytes([
            *start_value.get(offset).unwrap_or(&0),
            *start_value.get(offset + 1).unwrap_or(&0),
        ]);
        pit.start[channel] = Instant::now();
    }
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
        if !UART_OUTPUT_ENABLED.load(Ordering::Relaxed) {
            return;
        }
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
    let _ = pit_poll();
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
    if let Some(value) = rtc_read(port) {
        value
    } else if let Some(value) = pit_read(port) {
        value
    } else if let Some(value) = ps2_read(port) {
        value
    } else if let Some(value) = native_devices::io_read8(port) {
        value
    } else if (0x3F8..=0x3FF).contains(&port) {
        uart_read(port)
    } else {
        0xFF
    }
}

#[no_mangle]
pub extern "C" fn io_port_read16(port: i32) -> i32 {
    native_devices::io_read16(port).unwrap_or(0xFFFF)
}

#[no_mangle]
pub extern "C" fn io_port_read32(port: i32) -> i32 {
    native_devices::io_read32(port).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn io_port_write8(port: i32, value: i32) {
    if !rtc_write(port, value)
        && !pit_write(port, value)
        && !ps2_write(port, value)
        && !native_devices::io_write8(port, value)
        && (0x3F8..=0x3FF).contains(&port)
    {
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
pub extern "C" fn mmap_read8(addr: u32) -> i32 {
    if let Some(offset) = legacy_vga_offset(addr) {
        return vga_text_memory()
            .lock()
            .ok()
            .and_then(|m| m.get(offset).copied())
            .unwrap_or(0xFF) as i32;
    }
    native_devices::mmio_read8(addr).unwrap_or(0xFF)
}

#[no_mangle]
pub extern "C" fn mmap_read32(addr: u32) -> i32 {
    if legacy_vga_offset(addr).is_some() && addr <= 0xBFFFC {
        return i32::from_le_bytes([
            mmap_read8(addr) as u8,
            mmap_read8(addr + 1) as u8,
            mmap_read8(addr + 2) as u8,
            mmap_read8(addr + 3) as u8,
        ]);
    }
    native_devices::mmio_read32(addr).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn mmap_write8(addr: u32, value: i32) {
    if let Some(offset) = legacy_vga_offset(addr) {
        if let Ok(mut memory) = vga_text_memory().lock() {
            if let Some(byte) = memory.get_mut(offset) {
                *byte = value as u8;
            }
        }
        return;
    }
    let _ = native_devices::mmio_write8(addr, value);
}

#[no_mangle]
pub extern "C" fn mmap_write16(addr: u32, value: i32) {
    if legacy_vga_offset(addr).is_some() {
        mmap_write8(addr, value);
        mmap_write8(addr + 1, value >> 8);
        return;
    }
    let _ = native_devices::mmio_write16(addr, value);
}

#[no_mangle]
pub extern "C" fn mmap_write32(addr: u32, value: i32) {
    if legacy_vga_offset(addr).is_some() {
        for offset in 0..4 {
            mmap_write8(addr + offset, value >> (offset * 8));
        }
        return;
    }
    let _ = native_devices::mmio_write32(addr, value);
}

#[no_mangle]
pub extern "C" fn mmap_write64(addr: u32, v0: i32, v1: i32) {
    mmap_write32(addr, v0);
    mmap_write32(addr + 4, v1);
}

#[no_mangle]
pub extern "C" fn mmap_write128(addr: u32, v0: i32, v1: i32, v2: i32, v3: i32) {
    mmap_write32(addr, v0);
    mmap_write32(addr + 4, v1);
    mmap_write32(addr + 8, v2);
    mmap_write32(addr + 12, v3);
}

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
    screen_width: u32,
    screen_height: u32,
    screen_bpp: u32,
    graphical_mode: bool,
}

impl NativeCpu {
    pub fn new(ram_bytes: u32, vga_bytes: u32) -> Self {
        assert!(ram_bytes > 0, "RAM size must be non-zero");
        assert!(vga_bytes > 0, "VGA memory size must be non-zero");

        let mut state_arena = Box::new([0u8; 4096]);
        if let Ok(mut text) = vga_text_memory().lock() {
            text.fill(0);
        }
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
            screen_width: 80,
            screen_height: 25,
            screen_bpp: 0,
            graphical_mode: false,
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
                let pit_active = pit_poll();
                if *global_pointers::acpi_enabled {
                    let _ = apic::apic_timer(now);
                    cpu::handle_irqs();
                } else if !pit_active {
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

    /// Return the restored SVGA framebuffer as packed RGB bytes.
    /// The native runtime keeps the framebuffer in the same guest-visible
    /// backing store used by v86's LFB mapping.
    pub fn vga_text_snapshot(&self) -> Option<(u32, u32, Vec<u8>)> {
        if self.graphical_mode {
            return None;
        }
        let memory = vga_text_memory().lock().ok()?;
        if memory.len() < 80 * 25 * 2 {
            return None;
        }
        Some((80, 25, memory[..80 * 25 * 2].to_vec()))
    }

    pub fn vga_framebuffer_rgb(&self) -> Option<(u32, u32, Vec<u8>)> {
        if !self.graphical_mode || self.screen_width == 0 || self.screen_height == 0 {
            return None;
        }
        let pixels = (self.screen_width as usize).checked_mul(self.screen_height as usize)?;
        let mut output = vec![0u8; pixels.checked_mul(3)?];
        unsafe {
            if memory::vga_mem8.is_null() || self.screen_bpp != 32 {
                return None;
            }
            let source_len = pixels.checked_mul(4)?;
            if source_len > self.vga_bytes as usize {
                return None;
            }
            let source = std::slice::from_raw_parts(memory::vga_mem8, source_len);
            for (index, rgb) in output.chunks_exact_mut(3).enumerate() {
                let pixel = &source[index * 4..index * 4 + 4];
                rgb.copy_from_slice(&[pixel[2], pixel[1], pixel[0]]);
            }
        }
        Some((self.screen_width, self.screen_height, output))
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
        if let Some(rtc_state) = slots.get(47).and_then(serde_json::Value::as_array) {
            restore_rtc_state(rtc_state, buffers)?;
        }
        if let Some(pit_state) = slots.get(58).and_then(serde_json::Value::as_array) {
            restore_pit_state(pit_state, buffers)?;
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

        if let Some(vga_state) = slots.get(52).and_then(serde_json::Value::as_array) {
            self.screen_width = vga_state
                .get(15)
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as u32;
            self.screen_height = vga_state
                .get(16)
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as u32;
            self.screen_bpp = vga_state
                .get(19)
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as u32;
            self.graphical_mode = vga_state
                .get(9)
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if let Some(value) = vga_state.get(39) {
                let buffer_id = value
                    .get("buffer_id")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or_else(|| "VGA state[39] is not a typed buffer".to_owned())?
                    as usize;
                let svga = buffers
                    .get(buffer_id)
                    .ok_or_else(|| format!("VGA buffer id {buffer_id} is out of range"))?;
                let vga_len = self.vga_bytes as usize;
                if svga.len() > vga_len {
                    return Err(format!(
                        "VGA framebuffer {} exceeds allocated {} bytes",
                        svga.len(),
                        vga_len
                    ));
                }
                unsafe {
                    std::ptr::copy_nonoverlapping(svga.as_ptr(), memory::vga_mem8, svga.len());
                }
            }
            if vga_state.get(6).is_some() {
                let text = nested_buffer(vga_state, 6, buffers)?;
                let mut target = vga_text_memory()
                    .lock()
                    .map_err(|_| "VGA text mutex poisoned".to_owned())?;
                let copy_len = text.len().min(target.len());
                target[..copy_len].copy_from_slice(&text[..copy_len]);
                if copy_len < target.len() {
                    target[copy_len..].fill(0);
                }
            }
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

#[cfg(test)]
mod keyboard_tests {
    use super::keycode_for_ascii;

    #[test]
    fn maps_lowercase_without_shift() {
        assert_eq!(keycode_for_ascii(b'a'), Some((0x1E, false)));
        assert_eq!(keycode_for_ascii(b'z'), Some((0x2C, false)));
    }

    #[test]
    fn maps_uppercase_and_punctuation_with_shift() {
        assert_eq!(keycode_for_ascii(b'A'), Some((0x1E, true)));
        assert_eq!(keycode_for_ascii(b'!'), Some((0x02, true)));
        assert_eq!(keycode_for_ascii(b'_'), Some((0x0C, true)));
    }

    #[test]
    fn maps_shell_control_characters() {
        assert_eq!(keycode_for_ascii(b' '), Some((0x39, false)));
        assert_eq!(keycode_for_ascii(b'\n'), Some((0x1C, false)));
        assert_eq!(keycode_for_ascii(b'\t'), Some((0x0F, false)));
    }
}
