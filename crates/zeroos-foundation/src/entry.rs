// - `#[no_mangle] fn main() -> !` (Rust, must call exit())
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
        pub unsafe extern "C" fn __main_entry(argc: i32, argv: *const *const u8, envp: *const *const u8) -> i32 {
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
        pub extern "C" fn __main_entry(_argc: i32, _argv: *const *const u8, _envp: *const *const u8) -> i32 {
            debug::writeln!("[BOOT] __main_entry argc={} argv=0x{:x}", _argc, _argv as usize);

            unsafe {
                main()
                // Never returns - main() must call exit()
            }
        }
    }
}
