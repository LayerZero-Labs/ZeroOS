//! Frame pointer-based stack unwinding.
//!
//! This implementation walks the stack by following the frame pointer chain.
//! It's lightweight and works in no_std environments without external dependencies.
//!
//! ## Requirements
//!
//! - Build with `-Cforce-frame-pointers=yes`
//! - RISC-V architecture (riscv32 or riscv64)
//!
//! ## Trade-offs
//!
//! - ✓ Simple, no external dependencies
//! - ✓ Small overhead (~2-5 KB)
//! - ✗ May miss tail-call optimized frames
//! - ✗ Less accurate with aggressive optimizations

use crate::BacktraceCapture;
use core::fmt::Write;

/// Maximum number of stack frames to print
const MAX_FRAMES: usize = 64;

/// Frame pointer-based backtrace implementation.
pub struct FramePointerBacktrace;

impl BacktraceCapture for FramePointerBacktrace {
    fn init() {
        // In std mode, set custom panic hook to use frame pointer walking
        // instead of Rust's default DWARF-based std::backtrace
        #[cfg(not(target_os = "none"))]
        {
            extern crate std;
            use std::boxed::Box;
            use std::panic;
            use std::string::String;
            use std::sync::Once;

            static INIT: Once = Once::new();

            INIT.call_once(|| {
                panic::set_hook(Box::new(|info| {
                    // Extract panic message
                    let msg = if let Some(&s) = info.payload().downcast_ref::<&str>() {
                        s
                    } else if let Some(s) = info.payload().downcast_ref::<String>() {
                        s.as_str()
                    } else {
                        ""
                    };

                    // Print panic info
                    if let Some(location) = info.location() {
                        if !msg.is_empty() {
                            std::eprintln!("\nthread panicked at {}:\n{}", location, msg);
                        } else {
                            std::eprintln!("\nthread panicked at {}", location);
                        }
                    } else {
                        std::eprintln!("\nthread panicked");
                    }

                    // Print backtrace using frame pointer walking
                    unsafe {
                        FramePointerBacktrace::print_backtrace();
                    }
                }));
            });
        }

        // no_std mode: No initialization needed (panic handler calls print_backtrace directly)
    }

    #[inline(never)]
    unsafe fn print_backtrace() {
        // Safety: get_frame_pointer returns the current valid frame pointer
        unsafe { print_backtrace_from_fp(get_frame_pointer()) };
    }
}

/// Print a stack backtrace starting from a given frame pointer.
///
/// # Safety
///
/// The caller must ensure `fp` is a valid frame pointer from the current stack.
unsafe fn print_backtrace_from_fp(mut fp: *const usize) {
    // Use a simple writer that outputs to platform stdout
    struct StdoutWriter;

    impl Write for StdoutWriter {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            unsafe extern "C" {
                fn __platform_stdout_write(buf: *const u8, len: usize) -> isize;
            }
            unsafe {
                __platform_stdout_write(s.as_ptr(), s.len());
            }
            Ok(())
        }
    }

    let mut w = StdoutWriter;

    let _ = writeln!(w, "stack backtrace:");

    let mut frame_num = 0usize;

    while !fp.is_null() && frame_num < MAX_FRAMES {
        // Safety: We check fp is not null
        let ra = unsafe { read_return_address(fp) };

        if ra == 0 {
            break;
        }

        // Format matches Rust stdlib backtrace: "  N:         0xADDR - <unknown>"
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
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
#[inline(always)]
fn get_frame_pointer() -> *const usize {
    let fp: usize;
    unsafe {
        core::arch::asm!("mv {}, s0", out(reg) fp, options(nomem, nostack));
    }
    fp as *const usize
}

#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
#[inline(always)]
fn get_frame_pointer() -> *const usize {
    core::ptr::null()
}

/// Read the return address from a frame.
///
/// On RISC-V with standard calling convention:
/// - fp points to saved fp from caller
/// - fp - wordsize contains return address (ra)
#[inline(always)]
unsafe fn read_return_address(fp: *const usize) -> usize {
    let ra_ptr = (fp as usize).wrapping_sub(core::mem::size_of::<usize>()) as *const usize;
    if ra_ptr.is_null() {
        return 0;
    }
    unsafe { core::ptr::read_volatile(ra_ptr) }
}

/// Read the previous frame pointer from current frame.
#[inline(always)]
unsafe fn read_previous_fp(fp: *const usize) -> *const usize {
    let prev_fp_ptr = (fp as usize).wrapping_sub(2 * core::mem::size_of::<usize>()) as *const usize;
    if prev_fp_ptr.is_null() {
        return core::ptr::null();
    }
    unsafe { core::ptr::read_volatile(prev_fp_ptr) as *const usize }
}
