#![cfg_attr(not(feature = "std"), no_std)]

mod boot;
#[cfg(all(
    not(target_os = "none"),
    any(target_arch = "riscv32", target_arch = "riscv64")
))]
mod trap;

extern crate zeroos;

// Platform ABI symbols:
// - Mandatory:
//   - `__platform_bootstrap()` (in `boot.rs`): platform init hook called by arch bootstrap.
//   - `trap_handler(..)` (in `trap.rs`): required on RISC-V targets.
//   - `__platform_exit(..)`: used by `foundation::kfn::kexit` / platform `exit()`.
//   - `__platform_stdout_write(..)`: fundamental output primitive, used by panic handler.
// - Optional:
//   - `__debug_write(..)`: only required when the `debug` crate is enabled/linked.

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        pub use std::{eprintln, println};

        pub fn exit(code: i32) -> ! {
            std::process::exit(code)
        }
    } else {

        pub use htif::{eprintln, println, putchar};

        pub fn exit(code: i32) -> ! {
            __platform_exit(code)
        }

        #[cfg(all(feature = "memory", target_os = "none"))]
        #[global_allocator]
        static ALLOCATOR: zeroos::alloc::System = zeroos::alloc::System;

        #[cfg(target_os = "none")]
        #[panic_handler]
        fn panic(info: &core::panic::PanicInfo) -> ! {
            eprintln!("PANIC: {}", info);

            #[cfg(feature = "backtrace")]
            zeroos::runtime_nostd::backtrace::print_backtrace();

            // Exit with SIGABRT (128 + 6 = 134) per Linux convention
            __platform_abort(6)
        }
    }
}

#[no_mangle]
pub extern "C" fn __platform_exit(code: i32) -> ! {
    htif::exit(code as u32)
}

/// Abort the program with Linux-standard signal exit code.
///
/// This is called by the panic handler (via `zeroos-runtime-nostd`) or
/// signal handlers to terminate due to a signal.
///
/// # Arguments
/// * `sig` - The signal number (e.g., SIGABRT=6)
///
/// Exit code is computed as `128 + sig` per Linux convention.
/// For SIGABRT (6), this yields exit code 134.
#[no_mangle]
pub extern "C" fn __platform_abort(sig: i32) -> ! {
    __platform_exit(128 + sig)
}

#[no_mangle]
/// Platform stdout write - the fundamental output primitive.
///
/// This is always available since panic/error messages must be visible
/// regardless of debug feature state.
///
/// # Safety
/// - `msg` must be either null (in which case nothing is written) or a valid pointer to `len`
///   bytes of readable memory.
pub unsafe extern "C" fn __platform_stdout_write(msg: *const u8, len: usize) {
    if !msg.is_null() && len > 0 {
        let slice = core::slice::from_raw_parts(msg, len);
        for &byte in slice {
            htif::putchar(byte);
        }
    }
}

#[no_mangle]
/// Debug write - alias for __platform_stdout_write for zeroos-debug crate.
///
/// Only compiled when `debug` feature is enabled.
///
/// # Safety
/// Same as `__platform_stdout_write`.
#[cfg(feature = "debug")]
pub unsafe extern "C" fn __debug_write(msg: *const u8, len: usize) {
    __platform_stdout_write(msg, len);
}
