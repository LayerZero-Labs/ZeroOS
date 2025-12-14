#![no_std]

use core::ptr::null_mut;
use vfs_core::{noop_close, noop_ioctl, noop_seek, FdEntry, FileOps};

fn zero_read(_file: *mut u8, buf: *mut u8, count: usize) -> isize {
    if count == 0 {
        return 0;
    }
    if buf.is_null() {
        return -(libc::EFAULT as isize);
    }

    unsafe {
        core::ptr::write_bytes(buf, 0, count);
    }

    count as isize
}

fn zero_write(_file: *mut u8, _buf: *const u8, count: usize) -> isize {
    count as isize
}

pub const ZERO_FOPS: FileOps = FileOps {
    read: zero_read,
    write: zero_write,
    release: noop_close,
    llseek: noop_seek,
    ioctl: noop_ioctl,
};

pub fn zero_factory() -> FdEntry {
    FdEntry {
        ops: &ZERO_FOPS,
        private_data: null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_read() {
        let mut buf = [0xFFu8; 64];
        let result = zero_read(null_mut(), buf.as_mut_ptr(), buf.len());
        assert_eq!(result, 64, "/dev/zero read should succeed");
        assert!(buf.iter().all(|&b| b == 0), "Buffer should be all zeros");
    }

    #[test]
    fn test_zero_write() {
        let buf = [0u8; 64];
        let result = zero_write(null_mut(), buf.as_ptr(), buf.len());
        assert_eq!(result, 64, "/dev/zero write should succeed");
    }
}
