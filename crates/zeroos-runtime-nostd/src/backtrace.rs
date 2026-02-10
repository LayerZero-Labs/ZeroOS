//! Stack backtrace support for no_std environments
//!
//! Provides frame pointer walking to capture stack traces during panics.
//! Output format matches Rust's stdlib backtrace format for compatibility
//! with host-side symbolization tools like `addr2line`.
//!
//! ## Usage
//!
//! 1. Build with frame pointers: `-Cforce-frame-pointers=yes`
//! 2. Keep debug symbols (don't strip)
//! 3. Use `cargo spike run --symbolize-backtrace` or manually:
//!    `addr2line -e <binary> <addr>`

use core::fmt::Write;

use crate::io::StdoutWriter;

/// Maximum number of stack frames to print
const MAX_FRAMES: usize = 32;

/// Print a stack backtrace starting from the current frame.
///
/// Requires building with `-Cforce-frame-pointers=yes` for accurate results.
/// Output format matches Rust's stdlib backtrace format so host tools can
/// resolve addresses using addr2line.
#[inline(never)]
pub fn print_backtrace() {
    // Safety: get_frame_pointer returns the current valid frame pointer
    unsafe { print_backtrace_from_fp(get_frame_pointer()) };
}

/// Print a stack backtrace starting from a given frame pointer.
///
/// This is useful when you want to capture a backtrace from a specific point,
/// such as from a signal handler or when the frame pointer has been saved.
///
/// # Arguments
/// * `fp` - The frame pointer to start walking from. Must be a valid frame
///   pointer obtained from [`get_frame_pointer`] or from the same stack.
///
/// # Safety
/// The caller must ensure `fp` is a valid frame pointer from the current
/// stack. Invalid frame pointers may cause undefined behavior.
///
/// # Example
/// ```ignore
/// let fp = get_frame_pointer();
/// // ... do some work ...
/// unsafe { print_backtrace_from_fp(fp); } // Print backtrace from saved point
/// ```
pub unsafe fn print_backtrace_from_fp(mut fp: *const usize) {
    let mut w = StdoutWriter;

    let _ = writeln!(w, "stack backtrace:");

    let mut frame_num = 0usize;

    while !fp.is_null() && frame_num < MAX_FRAMES {
        // Safety: We check fp is not null. The validity of the memory is
        // assumed based on frame pointer chain integrity.
        let ra = unsafe { read_return_address(fp) };

        if ra == 0 {
            break;
        }

        // Format matches Rust stdlib backtrace: "  N:         0xADDR - <unknown>"
        // This allows host tools to symbolize with addr2line
        let _ = writeln!(w, "  {frame_num}:         0x{ra:x} - <unknown>");

        // Move to previous frame
        fp = unsafe { read_previous_fp(fp) };
        frame_num += 1;
    }

    if frame_num == MAX_FRAMES {
        let _ = writeln!(w, "  ... (truncated)");
    }
}

/// Get the current frame pointer (s0/fp register on RISC-V).
///
/// Returns a pointer that can be passed to [`print_backtrace_from_fp`] to
/// print a backtrace starting from that point.
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
#[inline(always)]
pub fn get_frame_pointer() -> *const usize {
    let fp: usize;
    unsafe {
        core::arch::asm!("mv {}, s0", out(reg) fp, options(nomem, nostack));
    }
    fp as *const usize
}

#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
#[inline(always)]
pub fn get_frame_pointer() -> *const usize {
    core::ptr::null()
}

/// Read the return address from a frame.
///
/// On RISC-V with standard calling convention:
/// - fp points to saved fp from caller
/// - fp - wordsize contains return address (ra)
#[inline(always)]
unsafe fn read_return_address(fp: *const usize) -> usize {
    // Return address is stored at fp - 8 (for 64-bit) or fp - 4 (for 32-bit)
    let ra_ptr = (fp as usize).wrapping_sub(core::mem::size_of::<usize>()) as *const usize;
    if ra_ptr.is_null() {
        return 0;
    }
    core::ptr::read_volatile(ra_ptr)
}

/// Read the previous frame pointer from current frame.
#[inline(always)]
unsafe fn read_previous_fp(fp: *const usize) -> *const usize {
    // Previous fp is stored at current fp - 16 (64-bit) or fp - 8 (32-bit)
    let prev_fp_ptr = (fp as usize).wrapping_sub(2 * core::mem::size_of::<usize>()) as *const usize;
    if prev_fp_ptr.is_null() {
        return core::ptr::null();
    }
    core::ptr::read_volatile(prev_fp_ptr) as *const usize
}
