#![no_std]

use core::ptr::null_mut;
use vfs_core::{noop_close, noop_ioctl, noop_seek, FdEntry, FileOps};

fn null_read(_file: *mut u8, _buf: *mut u8, _count: usize) -> isize {
    0
}

fn null_write(_file: *mut u8, _buf: *const u8, count: usize) -> isize {
    count as isize
}

pub const NULL_FOPS: FileOps = FileOps {
    read: null_read,
    write: null_write,
    release: noop_close,
    llseek: noop_seek,
    ioctl: noop_ioctl,
};

pub fn null_factory() -> FdEntry {
    FdEntry {
        ops: &NULL_FOPS,
        private_data: null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_read() {
        let mut buf = [0u8; 64];
        let result = null_read(null_mut(), buf.as_mut_ptr(), buf.len());
        assert_eq!(result, 0, "/dev/null read should return EOF");
    }

    #[test]
    fn test_null_write() {
        let buf = [0u8; 64];
        let result = null_write(null_mut(), buf.as_ptr(), buf.len());
        assert_eq!(result, 64, "/dev/null write should succeed");
    }
}
