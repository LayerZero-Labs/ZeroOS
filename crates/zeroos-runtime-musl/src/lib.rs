#![no_std]

mod lock_override;
mod stack;

pub use stack::build_musl_stack;

// Re-export unified backtrace interface for public API
pub use zeroos_backtrace::{Backtrace, BacktraceCapture};

// Backtrace initialization hook (runs from .init_array before main)
// Each implementation handles its own initialization:
// - DWARF: Registers .eh_frame with libgcc
// - Frame-pointers (std): Sets custom panic hook
// - Frame-pointers (no_std): No-op
// - Off: No-op
mod backtrace {
    use zeroos_backtrace::BacktraceCapture;

    #[unsafe(no_mangle)]
    extern "C" fn __zeroos_init_backtrace() {
        // Polymorphic call - each implementation handles its own setup
        zeroos_backtrace::Backtrace::init();
    }

    // Place initialization function in .init_array
    #[used]
    #[unsafe(link_section = ".init_array")]
    static __ZEROOS_BACKTRACE_INIT: extern "C" fn() = __zeroos_init_backtrace;
}

#[cfg(target_arch = "riscv64")]
pub mod riscv64;
