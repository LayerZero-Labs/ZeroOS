#![allow(non_upper_case_globals)]

use cfg_if::cfg_if;
#[allow(unused_imports)]
use foundation::kfn;
use libc;

#[cfg(feature = "memory")]
pub mod memory;
#[cfg(feature = "random")]
pub mod random;
pub mod signal;
#[cfg(feature = "scheduler")]
pub mod thread;
#[cfg(feature = "vfs")]
pub mod vfs;

#[inline]
pub fn sys_unsupported() -> isize {
    -(libc::ENOSYS as isize)
}

#[inline]
pub fn sys_noop() -> isize {
    0
}

cfg_if! {
    if #[cfg(feature = "scheduler")] {
        #[inline]
        pub fn sys_exit(status: usize) -> isize {
            thread::sys_exit(status)
        }

        #[inline]
        pub fn sys_exit_group(status: usize) -> isize {
            thread::sys_exit_group(status)
        }
    } else {
        #[inline]
        pub fn sys_exit(status: usize) -> isize {
            kfn::kexit(status as i32)
        }

        #[inline]
        pub fn sys_exit_group(status: usize) -> isize {
            kfn::kexit(status as i32)
        }
    }
}
