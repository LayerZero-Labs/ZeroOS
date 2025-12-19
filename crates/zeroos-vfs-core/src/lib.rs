#![no_std]

pub use foundation::ops::VfsOps;

pub use libc::{
    S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK, S_IRGRP, S_IROTH,
    S_IRUSR, S_IRWXG, S_IRWXO, S_IRWXU, S_IWGRP, S_IWOTH, S_IWUSR, S_IXGRP, S_IXOTH, S_IXUSR,
};

mod vfs;

pub use vfs::*;

pub type Fd = i32;

pub type VfsResult<T> = Result<T, isize>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FileOps {
    pub read: fn(file: *mut u8, buf: *mut u8, count: usize) -> isize,
    pub write: fn(file: *mut u8, buf: *const u8, count: usize) -> isize,
    pub release: fn(file: *mut u8) -> isize,
    pub llseek: fn(file: *mut u8, offset: isize, whence: i32) -> isize,
    pub ioctl: fn(file: *mut u8, request: usize, arg: usize) -> isize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FdEntry {
    pub ops: &'static FileOps,
    pub private_data: *mut u8,
}

pub type DeviceFactory = fn() -> FdEntry;

pub fn noop_close(_file: *mut u8) -> isize {
    0
}

pub fn noop_seek(_file: *mut u8, _offset: isize, _whence: i32) -> isize {
    -(libc::ESPIPE as isize)
}

pub fn noop_ioctl(_file: *mut u8, _request: usize, _arg: usize) -> isize {
    -(libc::ENOTTY as isize)
}

pub fn noop_read(_file: *mut u8, _buf: *mut u8, _count: usize) -> isize {
    -(libc::EBADF as isize)
}

pub fn noop_write(_file: *mut u8, _buf: *const u8, _count: usize) -> isize {
    -(libc::EBADF as isize)
}
