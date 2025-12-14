#![no_std]

use core::ptr::null_mut;

use vfs_core::FileOps;

fn urandom_read(_file: *mut u8, buf: *mut u8, count: usize) -> isize {
    if count != 0 && buf.is_null() {
        return -(libc::EFAULT as isize);
    }
    unsafe { foundation::kfn::random::krandom(buf, count) }
}

fn urandom_write(_file: *mut u8, _buf: *const u8, _count: usize) -> isize {
    -(libc::EBADF as isize)
}

fn urandom_close(_file: *mut u8) -> isize {
    0
}

fn urandom_seek(_file: *mut u8, _offset: isize, _whence: i32) -> isize {
    -(libc::ESPIPE as isize)
}

fn urandom_ioctl(_file: *mut u8, _request: usize, _arg: usize) -> isize {
    -(libc::ENOTTY as isize)
}

pub const URANDOM_FOPS: FileOps = FileOps {
    read: urandom_read,
    write: urandom_write,
    release: urandom_close,
    llseek: urandom_seek,
    ioctl: urandom_ioctl,
};

pub fn urandom_factory() -> vfs_core::FdEntry {
    vfs_core::FdEntry {
        ops: &URANDOM_FOPS,
        private_data: null_mut(),
    }
}
