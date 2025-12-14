#[inline]
pub fn kinit() {
    unsafe { (crate::KERNEL.vfs.init)() }
}

#[inline]
pub fn kread(fd: i32, buf: *mut u8, count: usize) -> isize {
    unsafe { (crate::KERNEL.vfs.read)(fd, buf, count) }
}

#[inline]
pub fn kwrite(fd: i32, buf: *const u8, count: usize) -> isize {
    unsafe { (crate::KERNEL.vfs.write)(fd, buf, count) }
}

#[inline]
/// # Safety
/// `path` must be a valid NUL-terminated string.
pub unsafe fn kopen(path: *const u8, flags: i32, mode: u32) -> isize {
    (crate::KERNEL.vfs.open)(path, flags, mode)
}

#[inline]
pub fn kclose(fd: i32) -> isize {
    unsafe { (crate::KERNEL.vfs.close)(fd) }
}

#[inline]
pub fn klseek(fd: i32, offset: isize, whence: i32) -> isize {
    unsafe { (crate::KERNEL.vfs.lseek)(fd, offset, whence) }
}

#[inline]
pub fn kioctl(fd: i32, request: usize, arg: usize) -> isize {
    unsafe { (crate::KERNEL.vfs.ioctl)(fd, request, arg) }
}

#[inline]
pub fn kfstat(fd: i32, statbuf: *mut u8) -> isize {
    unsafe { (crate::KERNEL.vfs.fstat)(fd, statbuf) }
}
