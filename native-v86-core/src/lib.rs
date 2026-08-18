#[macro_use]
mod dbg;

#[macro_use]
mod paging;

pub mod cpu;
pub mod native_runtime;

pub mod js_api;
pub mod profiler;

#[cfg(not(feature = "native-interpreter"))]
mod analysis;
#[cfg(not(feature = "native-interpreter"))]
mod codegen;
mod config;
#[cfg(not(feature = "native-interpreter"))]
mod control_flow;
mod cpu_context;
mod gen;
#[cfg(feature = "native-interpreter")]
#[path = "jit_native.rs"]
mod jit;
#[cfg(not(feature = "native-interpreter"))]
mod jit;
#[cfg(not(feature = "native-interpreter"))]
mod jit_instructions;
#[cfg(not(feature = "native-interpreter"))]
mod leb;
mod modrm;
#[cfg(feature = "native-interpreter")]
#[path = "opstats_native.rs"]
mod opstats;
#[cfg(not(feature = "native-interpreter"))]
mod opstats;
mod page;
mod prefix;
mod regs;
mod softfloat;
mod state_flags;
#[cfg(not(feature = "native-interpreter"))]
mod wasmgen;
mod zstd;
