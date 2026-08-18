use crate::page::Page;
use crate::state_flags::CachedStateFlags;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct WasmTableIndex(pub u16);

impl WasmTableIndex {
    pub fn to_u16(self) -> u16 {
        self.0
    }
}

impl From<WasmTableIndex> for u32 {
    fn from(value: WasmTableIndex) -> Self {
        value.0 as u32
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct CachedCode {
    pub wasm_table_index: WasmTableIndex,
    pub initial_state: u16,
}

impl CachedCode {
    pub const NONE: Self = Self {
        wasm_table_index: WasmTableIndex(0),
        initial_state: 0,
    };
}

pub const WASM_TABLE_SIZE: u32 = 0;
pub const CHECK_JIT_STATE_INVARIANTS: bool = false;
pub const JIT_INSTR_BLOCK_BOUNDARY_FLAG: u32 = 1;

pub fn is_near_end_of_page(address: u32) -> bool {
    address & 0xFFF >= 0x1000 - 15
}

pub fn jit_find_cache_entry(_phys_address: u32, _state_flags: CachedStateFlags) -> CachedCode {
    CachedCode::NONE
}

pub fn update_tlb_code(_virt_page: Page, _phys_page: Page) {}
pub fn jit_increase_hotness_and_maybe_compile(
    _virt_addr: i32,
    _phys_addr: u32,
    _cs_offset: u32,
    _state_flags: CachedStateFlags,
    _instruction_count: u32,
) {
}
pub fn jit_clear_cache_js() {}
pub fn jit_dirty_page(_page: Page) {}
pub fn jit_dirty_cache_small(_start_addr: u32, _end_addr: u32) {}
pub fn jit_page_has_code(_page: Page) -> bool { false }
pub fn jit_page_has_wasm_table_index(_page: Page, _wasm_table_index: u16) -> bool { false }
pub fn check_missed_entry_points(_phys_address: u32, _state_flags: CachedStateFlags) {}
