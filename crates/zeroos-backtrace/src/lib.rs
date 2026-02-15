//! Unified backtrace abstraction for ZeroOS.
//!
//! This crate provides a trait-based interface for backtrace capture with multiple
//! implementation strategies, selected at compile time via `cfg(zeroos_backtrace = "...")`.
//!
//! # Backtrace Modes
//!
//! - **off**: No backtrace support (minimal binary size)
//! - **dwarf**: DWARF-based unwinding using `.eh_frame` tables (accurate, ~10-30 KB overhead)
//! - **frame_pointers**: Frame pointer walking (lightweight, ~2-5 KB overhead)
//!
//! # Usage
//!
//! The active implementation is selected at compile time via the build system:
//!
//! ```bash
//! cargo spike build --backtrace=dwarf
//! cargo spike build --backtrace=frame-pointers
//! cargo spike build --backtrace=off
//! ```
//!
//! In runtime code, use the unified `Backtrace` type:
//!
//! ```rust,ignore
//! use zeroos_backtrace::{Backtrace, BacktraceCapture};
//!
//! // Initialize (call during runtime startup)
//! Backtrace::init();
//!
//! // Capture backtrace (call from panic handler)
//! unsafe {
//!     Backtrace::print_backtrace();
//! }
//! ```

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

use cfg_if::cfg_if;

/// Common interface for all backtrace implementations.
pub trait BacktraceCapture {
    /// Initialize backtrace support.
    ///
    /// This is called once during runtime initialization, before main.
    /// For DWARF mode, this registers the `.eh_frame` section with libgcc.
    /// For frame pointer mode, this is a no-op.
    fn init();

    /// Capture and print the current backtrace to stdout.
    ///
    /// # Safety
    ///
    /// This function walks the stack by reading memory. The caller must ensure:
    /// - The stack is in a valid state
    /// - Frame pointers (if used) are correctly maintained
    /// - This is called from a valid execution context (e.g., panic handler)
    unsafe fn print_backtrace();
}

// Conditional module inclusion and re-exports based on compile-time configuration
cfg_if! {
    if #[cfg(zeroos_backtrace = "off")] {
        mod noop;
        pub use noop::NoopBacktrace as Backtrace;
    } else if #[cfg(zeroos_backtrace = "dwarf")] {
        mod dwarf;
        pub use dwarf::DwarfBacktrace as Backtrace;
    } else if #[cfg(zeroos_backtrace = "frame_pointers")] {
        mod frame_pointer;
        pub use frame_pointer::FramePointerBacktrace as Backtrace;
    } else {
        // Default to noop when no mode is explicitly set (for cargo check/test without build system)
        mod noop;
        pub use noop::NoopBacktrace as Backtrace;
    }
}
