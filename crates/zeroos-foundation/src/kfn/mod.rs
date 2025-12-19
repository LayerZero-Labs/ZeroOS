use cfg_if::cfg_if;

extern "C" {
    fn platform_exit(code: i32) -> !;
}

#[inline]
pub fn kexit(code: i32) -> ! {
    unsafe { platform_exit(code) }
}

pub mod thread;

cfg_if! {
    if #[cfg(feature = "memory")] {
        pub mod memory;
    } else {
        pub(crate) mod memory;
    }
}

cfg_if! {
    if #[cfg(feature = "scheduler")] {
        pub mod scheduler;
    } else {
        pub(crate) mod scheduler;
    }
}

cfg_if! {
    if #[cfg(feature = "vfs")] {
        pub mod vfs;
    } else {
        pub(crate) mod vfs;
    }
}

cfg_if! {
    if #[cfg(feature = "random")] {
        pub mod random;
    } else {
        pub(crate) mod random;
    }
}

cfg_if! {
    if #[cfg(feature = "arch")] {
        pub mod arch;
    } else {
        pub(crate) mod arch;
    }
}

cfg_if! {
    if #[cfg(feature = "trap")] {
        pub mod trap;
    } else {
        pub(crate) mod trap;
    }
}
