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
//   - `platform_exit(..)`: used by `foundation::kfn::kexit` / platform `exit()`.
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
            platform_exit(code)
        }

        #[cfg(all(feature = "memory", target_os = "none"))]
        #[global_allocator]
        static ALLOCATOR: zeroos::alloc::System = zeroos::alloc::System;

        #[cfg(target_os = "none")]
        #[panic_handler]
        fn panic(info: &core::panic::PanicInfo) -> ! {
            eprintln!("PANIC: {}", info);
            exit(1)
        }
    }
}

#[no_mangle]
pub extern "C" fn platform_exit(code: i32) -> ! {
    htif::exit(code as u32)
}

#[no_mangle]
/// # Safety
/// - `msg` must be either null (in which case nothing is written) or a valid pointer to `len`
///   bytes of readable memory.
pub unsafe extern "C" fn __debug_write(msg: *const u8, len: usize) {
    if !msg.is_null() && len > 0 {
        let slice = core::slice::from_raw_parts(msg, len);
        for &byte in slice {
            htif::putchar(byte);
        }
    }
}
