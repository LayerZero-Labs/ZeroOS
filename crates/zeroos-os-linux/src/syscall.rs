#![allow(non_upper_case_globals)]

use cfg_if::cfg_if;
use foundation::SyscallFrame;
use libc::{self, c_long, *};

use crate::handlers;
#[cfg(feature = "scheduler")]
use crate::handlers::HandlerContext;

#[inline(always)]
fn with_ret<Frame, F>(regs: *mut Frame, f: F)
where
    Frame: SyscallFrame,
    F: FnOnce(&Frame) -> isize,
{
    let r = unsafe { &*regs };
    let ret = f(r);
    unsafe { (*regs).set_ret(ret) }
}

macro_rules! usize_ty {
    ($ignored:tt) => {
        usize
    };
}

macro_rules! define_call {
    ($name:ident ()) => {
        #[allow(dead_code)]
        #[inline(always)]
        fn $name<Frame, F>(regs: *mut Frame, f: F)
        where
            Frame: SyscallFrame,
            F: FnOnce() -> isize,
        {
            with_ret(regs, |_| f())
        }
    };
    ($name:ident ($($idx:tt),+)) => {
        #[allow(dead_code)]
        #[inline(always)]
        fn $name<Frame, F>(regs: *mut Frame, f: F)
        where
            Frame: SyscallFrame,
            F: FnOnce($(usize_ty!($idx)),+) -> isize,
        {
            with_ret(regs, |r| f($(r.arg($idx)),+))
        }
    };
}

define_call!(call0());
define_call!(call1(0));
define_call!(call2(0, 1));
define_call!(call3(0, 1, 2));
define_call!(call4(0, 1, 2, 3));
define_call!(call5(0, 1, 2, 3, 4));
define_call!(call6(0, 1, 2, 3, 4, 5));

/// # Safety
/// `regs` must be a valid pointer to a syscall frame.
pub unsafe fn dispatch_syscall<Frame: SyscallFrame>(regs: *mut Frame) {
    let regs_ref = unsafe { &*regs };

    let nr = regs_ref.syscall_number();
    let nr = nr as c_long;

    cfg_if! { if #[cfg(feature = "memory")] {
        match nr {
            SYS_brk => return call1(regs, handlers::memory::sys_brk),
            SYS_mmap => return call6(regs, handlers::memory::sys_mmap),
            SYS_munmap => return call2(regs, handlers::memory::sys_munmap),
            SYS_mprotect => return call3(regs, handlers::memory::sys_mprotect),
            SYS_madvise => return call3(regs, |_addr, _len, _advice| 0),
            _ => {}
        }
    }}

    cfg_if! { if #[cfg(feature = "scheduler")] {
        let mepc = regs_ref.pc();
        let frame_ptr = regs as usize;
        let ctx = HandlerContext::new(mepc, frame_ptr);
        match nr {
            SYS_clone => return call5(regs, |flags, stack, parent_tid, tls, child_tid| {
                handlers::thread::sys_clone(flags, stack, parent_tid, tls, child_tid, &ctx)
            }),
            SYS_futex => return call3(regs, |addr, op, val| {
                handlers::thread::sys_futex(addr, op, val, &ctx)
            }),
            SYS_sched_yield => return call0(regs, || handlers::thread::sys_sched_yield(&ctx)),
            SYS_getpid => return call0(regs, handlers::thread::sys_getpid),
            SYS_gettid => return call0(regs, handlers::thread::sys_gettid),
            SYS_set_tid_address => return call1(regs, handlers::thread::sys_set_tid_address),
            _ => {}
        }
    }}

    cfg_if! { if #[cfg(feature = "vfs")] {
        match nr {
            SYS_openat => return call4(regs, handlers::vfs::sys_openat),
            SYS_close => return call1(regs, handlers::vfs::sys_close),
            SYS_read => return call3(regs, handlers::vfs::sys_read),
            SYS_write => return call3(regs, handlers::vfs::sys_write),
            SYS_readv => return call3(regs, handlers::vfs::sys_readv),
            SYS_writev => return call3(regs, handlers::vfs::sys_writev),
            SYS_lseek => return call3(regs, handlers::vfs::sys_lseek),
            SYS_ioctl => return call3(regs, handlers::vfs::sys_ioctl),
            SYS_fstat => return call2(regs, handlers::vfs::sys_fstat),
            _ => {}
        }
    }}

    cfg_if! { if #[cfg(feature = "random")] {
        if nr == SYS_getrandom {
            return call3(regs, handlers::random::sys_getrandom);
        }
    }}

    match nr {
        SYS_exit => return call1(regs, handlers::sys_exit),
        SYS_exit_group => return call1(regs, handlers::sys_exit_group),
        _ => {}
    }
    call0(regs, handlers::sys_unsupported)
}
