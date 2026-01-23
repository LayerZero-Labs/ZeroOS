// Entry point for calling the user's main() function.
//
// `__main_entry` is defined as a weak symbol that jumps to `__default_main_entry`.
// Platforms/SDKs can provide their own strong `__main_entry` to override this.

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
use core::arch::global_asm;

unsafe extern "C" {
    /// # Safety
    /// Entry point for the user's main function.
    /// This is a weak symbol defined by foundation that can be overridden
    /// by platforms/SDKs that need a different main() signature.
    pub unsafe fn __main_entry(argc: i32, argv: *const *const u8, envp: *const *const u8) -> i32;
}

cfg_if::cfg_if! {
    if #[cfg(feature = "libc-main")] {
        extern "C" {
            fn main(argc: i32, argv: *const *const u8, envp: *const *const u8) -> i32;
        }

        #[no_mangle]
        #[inline(never)]
        /// # Safety
        /// The caller must provide `argv` and `envp` pointers that are valid per the platform ABI
        /// (or null), and remain valid for the duration of the call.
        pub unsafe extern "C" fn __default_main_entry(argc: i32, argv: *const *const u8, envp: *const *const u8) -> i32 {
            main(argc, argv, envp)
        }
    } else {
        // Rust-style main (must call exit, never return)
        // Expected to never return (must call exit()).
        extern "Rust" {
            fn main() -> !;
        }

        #[no_mangle]
        #[inline(never)]
        pub extern "C" fn __default_main_entry(_argc: i32, _argv: *const *const u8, _envp: *const *const u8) -> i32 {
            debug::writeln!("[BOOT] __main_entry argc={} argv=0x{:x}", _argc, _argv as usize);

            unsafe {
                main()
                // Never returns - main() must call exit()
            }
        }
    }
}

// Define __main_entry as a weak symbol that jumps to __default_main_entry.
// Platforms providing their own __main_entry can define a strong symbol to override.
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
global_asm!(
    ".weak __main_entry",
    ".type __main_entry, @function",
    "__main_entry:",
    "j {default}",
    default = sym __default_main_entry,
);
