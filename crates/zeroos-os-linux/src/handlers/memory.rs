use core::alloc::Layout;

use foundation::kfn;
use libc;

const PAGE_SIZE: usize = 4096;

pub fn sys_brk(_brk: usize) -> isize {
    -(libc::ENOMEM as isize)
}

pub fn sys_mmap(
    addr: usize,
    len: usize,
    prot: usize,
    flags: usize,
    fd: usize,
    offset: usize,
) -> isize {
    if len == 0 {
        return -(libc::EINVAL as isize);
    }
    let allowed_prot = (libc::PROT_NONE | libc::PROT_READ | libc::PROT_WRITE) as usize;
    if (prot & !allowed_prot) != 0 {
        return -(libc::EINVAL as isize);
    }

    let allowed_flags = (libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_STACK) as usize;
    if (flags & !allowed_flags) != 0 {
        return -(libc::EINVAL as isize);
    }
    if (flags & libc::MAP_PRIVATE as usize) == 0 || (flags & libc::MAP_ANONYMOUS as usize) == 0 {
        return -(libc::EINVAL as isize);
    }
    if addr != 0 || offset != 0 {
        return -(libc::EINVAL as isize);
    }
    if fd != usize::MAX && fd != 0 {
        return -(libc::EINVAL as isize);
    }

    let pages = len.div_ceil(PAGE_SIZE);
    let size = match pages.checked_mul(PAGE_SIZE) {
        Some(s) => s,
        None => return -(libc::EINVAL as isize),
    };
    let layout = match Layout::from_size_align(size, PAGE_SIZE) {
        Ok(l) => l,
        Err(_) => return -(libc::EINVAL as isize),
    };
    let ptr = kfn::memory::kmalloc(layout);
    if ptr.is_null() {
        return -(libc::ENOMEM as isize);
    }
    unsafe {
        core::ptr::write_bytes(ptr, 0, size);
    }
    ptr as isize
}

pub fn sys_munmap(addr: usize, len: usize) -> isize {
    if addr == 0 || len == 0 {
        return -(libc::EINVAL as isize);
    }
    if !addr.is_multiple_of(PAGE_SIZE) {
        return -(libc::EINVAL as isize);
    }
    let pages = len.div_ceil(PAGE_SIZE);
    let size = match pages.checked_mul(PAGE_SIZE) {
        Some(s) => s,
        None => return -(libc::EINVAL as isize),
    };
    let layout = match Layout::from_size_align(size, PAGE_SIZE) {
        Ok(l) => l,
        Err(_) => return -(libc::EINVAL as isize),
    };
    kfn::memory::kfree(addr as *mut u8, layout);
    0
}

pub fn sys_mprotect(addr: usize, len: usize, prot: usize) -> isize {
    if addr == 0 || len == 0 {
        return -(libc::EINVAL as isize);
    }
    if !addr.is_multiple_of(PAGE_SIZE) {
        return -(libc::EINVAL as isize);
    }
    let allowed_prot = (libc::PROT_NONE | libc::PROT_READ | libc::PROT_WRITE) as usize;
    if (prot & !allowed_prot) != 0 {
        return -(libc::EINVAL as isize);
    }
    0
}
