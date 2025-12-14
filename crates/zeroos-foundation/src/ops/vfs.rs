#[derive(Clone, Copy)]
pub struct VfsOps {
    pub init: fn(),
    pub read: fn(fd: i32, buf: *mut u8, count: usize) -> isize,
    pub write: fn(fd: i32, buf: *const u8, count: usize) -> isize,
    pub open: unsafe fn(path: *const u8, flags: i32, mode: u32) -> isize,
    pub close: fn(fd: i32) -> isize,
    pub lseek: fn(fd: i32, offset: isize, whence: i32) -> isize,
    pub ioctl: fn(fd: i32, request: usize, arg: usize) -> isize,
    pub fstat: fn(fd: i32, statbuf: *mut u8) -> isize,
}
