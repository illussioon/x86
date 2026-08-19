//! Cross-platform native x86 machine library.
//!
//! The crate intentionally exposes platform-neutral Rust APIs. It does not
//! require a browser, WebAssembly runtime, DOM, or platform-specific GUI.

pub mod bootloader;
pub mod error;
pub mod image;
pub mod machine;
pub mod state;

#[cfg(feature = "native-runtime")]
pub mod native_backend;

pub use bootloader::{Bootloader, FetchOptions, Resource, copy_resource_to, load_resource};
pub use error::{Result, X86Error};
pub use image::{Image, ImageKind};
pub use machine::{
    ConsoleConfig, ConsoleMode, ExecutionBackend, Machine, MachineConfig, MachineStatus,
    RunOptions, RunReport,
};
pub use state::{SavedState, StateHeader, StateSummary};

#[cfg(feature = "native-runtime")]
pub use native_backend::NativeBackend;

pub const API_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn native_capabilities() -> &'static [&'static str] {
    &[
        "raw-image-memory",
        "raw-image-file",
        "bootloader-file",
        "bootloader-http",
        "bootloader-https",
        "saved-state-v86-v6",
        "console-host-api",
        "vga-framebuffer-ppm",
        "ps2-keyboard-input",
        "terminal-ansi-renderer",
        "persistent-terminal-console",
        "script-download-and-exec",
        "host-process-backgrounding",
        "backend-trait",
        "native-v86-interpreter",
        "no-webassembly-runtime",
    ]
}
