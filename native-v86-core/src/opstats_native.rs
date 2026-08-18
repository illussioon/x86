#[allow(dead_code)]
pub fn record_opstat_jit_exit(_opcode: u32) {}

#[allow(dead_code)]
pub fn record_opstat_compiled(_opcode: u32) {}

#[allow(dead_code)]
pub fn record_opstat_size_wasm(_opcode: u32, _size: u64) {}

#[allow(dead_code)]
pub fn gen_opstat_unguarded_register<T>(_builder: &mut T, _opcode: u32) {}

#[allow(dead_code)]
pub fn gen_opstats<T>(_builder: &mut T, _opcode: u32) {}
