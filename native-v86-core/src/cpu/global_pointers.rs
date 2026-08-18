#![allow(non_upper_case_globals)]

use crate::cpu::cpu::reg128;
use crate::softfloat::F80;
use crate::state_flags::CachedStateFlags;
use std::ptr;

// v86 stores CPU scalar state in a compact fixed layout. WASM used address 0 as
// the linear-memory base; native builds initialize these pointers against an
// owned heap allocation instead.
pub static mut reg8: *mut u8 = ptr::null_mut();
pub static mut reg16: *mut u16 = ptr::null_mut();
pub static mut reg32: *mut i32 = ptr::null_mut();
pub static mut last_op_size: *mut i32 = ptr::null_mut();
pub static mut flags_changed: *mut i32 = ptr::null_mut();
pub static mut last_op1: *mut i32 = ptr::null_mut();
pub static mut state_flags: *mut CachedStateFlags = ptr::null_mut();
pub static mut last_result: *mut i32 = ptr::null_mut();
pub static mut flags: *mut i32 = ptr::null_mut();
pub static mut segment_access_bytes: *mut u8 = ptr::null_mut();
pub static mut apic_enabled: *mut bool = ptr::null_mut();
pub static mut acpi_enabled: *mut bool = ptr::null_mut();
pub static mut instruction_pointer: *mut i32 = ptr::null_mut();
pub static mut previous_ip: *mut i32 = ptr::null_mut();
pub static mut idtr_size: *mut i32 = ptr::null_mut();
pub static mut idtr_offset: *mut i32 = ptr::null_mut();
pub static mut gdtr_size: *mut i32 = ptr::null_mut();
pub static mut gdtr_offset: *mut i32 = ptr::null_mut();
pub static mut cr: *mut i32 = ptr::null_mut();
pub static mut cpl: *mut u8 = ptr::null_mut();
pub static mut in_hlt: *mut bool = ptr::null_mut();
pub static mut last_virt_eip: *mut i32 = ptr::null_mut();
pub static mut eip_phys: *mut i32 = ptr::null_mut();
pub static mut sysenter_cs: *mut i32 = ptr::null_mut();
pub static mut sysenter_esp: *mut i32 = ptr::null_mut();
pub static mut sysenter_eip: *mut i32 = ptr::null_mut();
pub static mut prefixes: *mut u8 = ptr::null_mut();
pub static mut instruction_counter: *mut u32 = ptr::null_mut();
pub static mut sreg: *mut u16 = ptr::null_mut();
pub static mut dreg: *mut i32 = ptr::null_mut();
pub static mut svga_dirty_bitmap_min_offset: *mut u32 = ptr::null_mut();
pub static mut svga_dirty_bitmap_max_offset: *mut u32 = ptr::null_mut();
pub static mut segment_is_null: *mut bool = ptr::null_mut();
pub static mut segment_offsets: *mut i32 = ptr::null_mut();
pub static mut segment_limits: *mut u32 = ptr::null_mut();
pub static mut protected_mode: *mut bool = ptr::null_mut();
pub static mut is_32: *mut bool = ptr::null_mut();
pub static mut stack_size_32: *mut bool = ptr::null_mut();
pub static mut memory_size: *mut u32 = ptr::null_mut();
pub static mut fpu_stack_empty: *mut u8 = ptr::null_mut();
pub static mut mxcsr: *mut i32 = ptr::null_mut();
pub static mut reg_xmm: *mut reg128 = ptr::null_mut();
pub static mut current_tsc: *mut u64 = ptr::null_mut();
pub static mut reg_pdpte: *mut u64 = ptr::null_mut();
pub static mut fpu_stack_ptr: *mut u8 = ptr::null_mut();
pub static mut fpu_control_word: *mut u16 = ptr::null_mut();
pub static mut fpu_status_word: *mut u16 = ptr::null_mut();
pub static mut fpu_opcode: *mut i32 = ptr::null_mut();
pub static mut fpu_ip: *mut i32 = ptr::null_mut();
pub static mut fpu_ip_selector: *mut i32 = ptr::null_mut();
pub static mut fpu_dp: *mut i32 = ptr::null_mut();
pub static mut fpu_dp_selector: *mut i32 = ptr::null_mut();
pub static mut tss_size_32: *mut bool = ptr::null_mut();
pub static mut sse_scratch_register: *mut reg128 = ptr::null_mut();
pub static mut fpu_st: *mut F80 = ptr::null_mut();

#[inline]
unsafe fn at<T>(base: *mut u8, offset: usize) -> *mut T {
    base.add(offset) as *mut T
}

/// Initialize the v86 CPU state pointer table against a native allocation.
/// The first 4 KiB must remain available for the scalar state layout.
pub unsafe fn init(base: *mut u8) {
    reg8 = at(base, 64);
    reg16 = at(base, 64);
    reg32 = at(base, 64);
    last_op_size = at(base, 96);
    flags_changed = at(base, 100);
    last_op1 = at(base, 104);
    state_flags = at(base, 108);
    last_result = at(base, 112);
    flags = at(base, 120);
    segment_access_bytes = at(base, 512);
    apic_enabled = at(base, 548);
    acpi_enabled = at(base, 552);
    instruction_pointer = at(base, 556);
    previous_ip = at(base, 560);
    idtr_size = at(base, 564);
    idtr_offset = at(base, 568);
    gdtr_size = at(base, 572);
    gdtr_offset = at(base, 576);
    cr = at(base, 580);
    cpl = at(base, 612);
    in_hlt = at(base, 616);
    last_virt_eip = at(base, 620);
    eip_phys = at(base, 624);
    sysenter_cs = at(base, 636);
    sysenter_esp = at(base, 640);
    sysenter_eip = at(base, 644);
    prefixes = at(base, 648);
    instruction_counter = at(base, 664);
    sreg = at(base, 668);
    dreg = at(base, 684);
    svga_dirty_bitmap_min_offset = at(base, 716);
    svga_dirty_bitmap_max_offset = at(base, 720);
    segment_is_null = at(base, 724);
    segment_offsets = at(base, 736);
    segment_limits = at(base, 768);
    protected_mode = at(base, 800);
    is_32 = at(base, 804);
    stack_size_32 = at(base, 808);
    memory_size = at(base, 812);
    fpu_stack_empty = at(base, 816);
    mxcsr = at(base, 824);
    reg_xmm = at(base, 832);
    current_tsc = at(base, 960);
    reg_pdpte = at(base, 968);
    fpu_stack_ptr = at(base, 1032);
    fpu_control_word = at(base, 1036);
    fpu_status_word = at(base, 1040);
    fpu_opcode = at(base, 1044);
    fpu_ip = at(base, 1048);
    fpu_ip_selector = at(base, 1052);
    fpu_dp = at(base, 1056);
    fpu_dp_selector = at(base, 1060);
    tss_size_32 = at(base, 1128);
    sse_scratch_register = at(base, 1136);
    fpu_st = at(base, 1152);
}

pub fn get_reg32_offset(r: u32) -> u32 {
    dbg_assert!(r < 8);
    unsafe { reg32.add(r as usize) as u32 }
}

pub fn get_reg_mmx_offset(r: u32) -> u32 {
    dbg_assert!(r < 8);
    unsafe { fpu_st.add(r as usize) as u32 }
}

pub fn get_reg_xmm_offset(r: u32) -> u32 {
    dbg_assert!(r < 8);
    unsafe { reg_xmm.add(r as usize) as u32 }
}

pub fn get_sreg_offset(s: u32) -> u32 {
    dbg_assert!(s < 6);
    unsafe { sreg.add(s as usize) as u32 }
}

pub fn get_seg_offset(s: u32) -> u32 {
    dbg_assert!(s < 8);
    unsafe { segment_offsets.add(s as usize) as u32 }
}

pub fn get_segment_is_null_offset(s: u32) -> u32 {
    dbg_assert!(s < 8);
    unsafe { segment_is_null.add(s as usize) as u32 }
}

pub fn get_creg_offset(i: u32) -> u32 {
    dbg_assert!(i < 8);
    unsafe { cr.add(i as usize) as u32 }
}
