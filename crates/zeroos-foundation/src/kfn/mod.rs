use cfg_if::cfg_if;

// Platform must provide this function
extern "C" {
    fn platform_exit(code: i32) -> !;
}

#[inline]
pub fn kexit(code: i32) -> ! {
    unsafe { platform_exit(code) }
}

cfg_if! {
    if #[cfg(feature = "memory")] {
        pub mod memory;
    }
}

cfg_if! {
    if #[cfg(feature = "scheduler")] {
        pub mod scheduler;
    }
}

cfg_if! {
    if #[cfg(feature = "vfs")] {
        pub mod vfs;
    }
}

cfg_if! {
    if #[cfg(feature = "random")] {
        pub mod random;
    }
}
