#![no_std]

extern crate alloc;

pub mod arch;
pub mod entry;
pub mod kernel;
pub mod kfn;
pub mod ops;
pub mod utils;

pub use arch::SyscallFrame;
pub use entry::__main_entry;

pub use kernel::{init, GlobalKernel, Kernel, KERNEL};

#[cfg(feature = "arch")]
pub use kernel::register_arch;
#[cfg(feature = "memory")]
pub use kernel::register_memory;
#[cfg(feature = "random")]
pub use kernel::register_random;
#[cfg(feature = "scheduler")]
pub use kernel::register_scheduler;
#[cfg(feature = "trap")]
pub use kernel::register_trap;
#[cfg(feature = "vfs")]
pub use kernel::register_vfs;
