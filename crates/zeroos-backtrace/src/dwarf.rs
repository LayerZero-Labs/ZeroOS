//! DWARF-based stack unwinding using `.eh_frame` tables.
//!
//! This implementation registers the `.eh_frame` section with libgcc's unwinder,
//! enabling Rust's standard backtrace machinery to work.
//!
//! ## Requirements
//!
//! - Build with `-Cforce-unwind-tables=yes`
//! - Linker script must preserve `.eh_frame` and `.eh_frame_hdr` sections
//! - libgcc must be linked (provides `__register_frame`)
//! - Environment variable `RUST_BACKTRACE=full` must be set
//!
//! ## How It Works
//!
//! 1. Compiler emits `.eh_frame` section containing Call Frame Information (CFI)
//! 2. Linker script preserves these sections (via `KEEP(.eh_frame)`)
//! 3. During runtime init (`.init_array`), we call libgcc's `__register_frame()`
//! 4. This registers the frame tables with libgcc's unwinder
//! 5. Rust's std::backtrace can now walk the stack using registered frames
//!
//! ## Trade-offs
//!
//! - ✓ Excellent accuracy (handles all optimizations)
//! - ✓ Platform-independent (DWARF is universal)
//! - ✗ Larger binary
//! - ✗ Requires libgcc

use crate::BacktraceCapture;

unsafe extern "C" {
    /// libgcc frame registration API (DWARF2 unwinder).
    ///
    /// Registers the `.eh_frame` section with libgcc's unwinding machinery.
    /// After registration, libgcc can use the frame tables for stack unwinding.
    fn __register_frame(begin: *const u8);

    /// Start of `.eh_frame` section (provided by linker script).
    static __eh_frame_start: u8;

    /// End of `.eh_frame` section (provided by linker script).
    static __eh_frame_end: u8;
}

/// DWARF-based backtrace implementation.
///
/// This implementation registers `.eh_frame` tables with libgcc during initialization.
/// Actual backtrace printing is handled by Rust's standard library when
/// `RUST_BACKTRACE=full` is set in the environment.
pub struct DwarfBacktrace;

impl BacktraceCapture for DwarfBacktrace {
    fn init() {
        // Register .eh_frame section with libgcc's unwinder
        let start = core::ptr::addr_of!(__eh_frame_start);
        let end = core::ptr::addr_of!(__eh_frame_end);

        // Only register if .eh_frame section is non-empty
        if start != end {
            unsafe {
                __register_frame(start);
            }
        }
    }

    unsafe fn print_backtrace() {
        // In std mode with RUST_BACKTRACE=full set, Rust's standard library
        // panic handler automatically prints backtraces using the registered
        // .eh_frame tables. We don't need to do anything here.
        //
        // Note: This function is called from panic handlers, but in std mode
        // the default panic hook has already printed the backtrace before
        // calling any custom panic handling code.
        //
        // For nostd mode with DWARF, manually walking .eh_frame is complex
        // and not currently implemented. Use frame-pointers mode instead.
    }
}

// NOTE: The .init_array hook (__zeroos_register_eh_frame and __ZEROOS_EH_FRAME_INIT)
// is defined in zeroos-runtime-musl/src/lib.rs, not here. This separation ensures
// that the runtime crate controls initialization timing and avoids duplicate symbols.
