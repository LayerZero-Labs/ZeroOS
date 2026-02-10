#![no_std]

use cfg_if::cfg_if;

// Basic I/O utilities (StdoutWriter)
pub mod io;

cfg_if! {
    if #[cfg(feature = "memory")] {
        pub mod alloc;
    }
}

cfg_if! {
    if #[cfg(target_arch = "riscv64")] {
        pub mod riscv64;
        pub use riscv64::__runtime_bootstrap;
    }
}

// Panic handler for no_std environments
// Platforms must provide __platform_abort() and __platform_stdout_write() symbols
// Disable with `default-features = false` to use a custom panic handler
#[cfg(feature = "panic")]
pub mod panic;

// Stack backtrace via frame pointer walking
#[cfg(feature = "backtrace")]
pub mod backtrace;
