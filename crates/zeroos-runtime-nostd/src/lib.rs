#![no_std]

use cfg_if::cfg_if;

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
