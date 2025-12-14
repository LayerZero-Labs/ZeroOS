#![no_std]

use vfs_core::{noop_close, noop_ioctl, noop_seek, FileOps};

fn console_read_eof(_file: *mut u8, _buf: *mut u8, _count: usize) -> isize {
    0
}

fn console_read_unsupported(_file: *mut u8, _buf: *mut u8, _count: usize) -> isize {
    -(libc::EBADF as isize)
}

fn console_write_unsupported(_file: *mut u8, _buf: *const u8, _count: usize) -> isize {
    -(libc::EBADF as isize)
}

pub const fn read_only_fops(read_fn: Option<fn(*mut u8, *mut u8, usize) -> isize>) -> FileOps {
    FileOps {
        read: if let Some(f) = read_fn {
            f
        } else {
            console_read_eof
        },
        write: console_write_unsupported,
        release: noop_close,
        llseek: noop_seek,
        ioctl: noop_ioctl,
    }
}

pub const fn write_only_fops(write_fn: fn(*mut u8, *const u8, usize) -> isize) -> FileOps {
    FileOps {
        read: console_read_unsupported,
        write: write_fn,
        release: noop_close,
        llseek: noop_seek,
        ioctl: noop_ioctl,
    }
}

pub use read_only_fops as stdin_fops;
pub use write_only_fops as stdout_fops;
pub use write_only_fops as stderr_fops;
