use foundation::kfn;
use libc;

pub fn sys_openat(_dirfd: usize, path: usize, flags: usize, mode: usize) -> isize {
    if path == 0 {
        return -(libc::EFAULT as isize);
    }
    unsafe { kfn::vfs::kopen(path as *const u8, flags as i32, mode as u32) }
}

pub fn sys_close(fd: usize) -> isize {
    kfn::vfs::kclose(fd as i32)
}

pub fn sys_read(fd: usize, buf: usize, count: usize) -> isize {
    if count == 0 {
        return 0;
    }
    if buf == 0 {
        return -(libc::EFAULT as isize);
    }
    kfn::vfs::kread(fd as i32, buf as *mut u8, count)
}

pub fn sys_write(fd: usize, buf: usize, count: usize) -> isize {
    if count == 0 {
        return 0;
    }
    if buf == 0 {
        return -(libc::EFAULT as isize);
    }
    kfn::vfs::kwrite(fd as i32, buf as *const u8, count)
}

#[repr(C)]
struct IoVec {
    iov_base: *mut u8,
    iov_len: usize,
}

pub fn sys_readv(fd: usize, iov: usize, iovcnt: usize) -> isize {
    if iovcnt == 0 {
        return -(libc::EINVAL as isize);
    }
    if iov == 0 {
        return -(libc::EFAULT as isize);
    }
    if iovcnt > (libc::UIO_MAXIOV as usize) {
        return -(libc::EINVAL as isize);
    }
    if !iov.is_multiple_of(core::mem::align_of::<IoVec>()) {
        return -(libc::EINVAL as isize);
    }
    let iovecs = unsafe { core::slice::from_raw_parts(iov as *const IoVec, iovcnt) };
    let mut total = 0isize;
    for v in iovecs {
        if v.iov_len == 0 {
            continue;
        }
        if v.iov_base.is_null() {
            return if total > 0 {
                total
            } else {
                -(libc::EFAULT as isize)
            };
        }
        let r = kfn::vfs::kread(fd as i32, v.iov_base, v.iov_len);
        if r < 0 {
            return if total > 0 { total } else { r };
        }
        total += r;
        if (r as usize) < v.iov_len {
            break;
        }
    }
    total
}

pub fn sys_writev(fd: usize, iov: usize, iovcnt: usize) -> isize {
    if iovcnt == 0 {
        return -(libc::EINVAL as isize);
    }
    if iov == 0 {
        return -(libc::EFAULT as isize);
    }
    if iovcnt > (libc::UIO_MAXIOV as usize) {
        return -(libc::EINVAL as isize);
    }
    if !iov.is_multiple_of(core::mem::align_of::<IoVec>()) {
        return -(libc::EINVAL as isize);
    }
    let iovecs = unsafe { core::slice::from_raw_parts(iov as *const IoVec, iovcnt) };
    let mut total = 0isize;
    for v in iovecs {
        if v.iov_len == 0 {
            continue;
        }
        if v.iov_base.is_null() {
            return if total > 0 {
                total
            } else {
                -(libc::EFAULT as isize)
            };
        }
        let r = kfn::vfs::kwrite(fd as i32, v.iov_base as *const u8, v.iov_len);
        if r < 0 {
            return if total > 0 { total } else { r };
        }
        total += r;
        if (r as usize) < v.iov_len {
            break;
        }
    }
    total
}

pub fn sys_lseek(fd: usize, offset: usize, whence: usize) -> isize {
    kfn::vfs::klseek(fd as i32, offset as isize, whence as i32)
}

pub fn sys_ioctl(fd: usize, request: usize, arg: usize) -> isize {
    kfn::vfs::kioctl(fd as i32, request, arg)
}

pub fn sys_fstat(fd: usize, statbuf: usize) -> isize {
    if statbuf == 0 {
        return -(libc::EFAULT as isize);
    }
    kfn::vfs::kfstat(fd as i32, statbuf as *mut u8)
}
