#![allow(non_upper_case_globals)]

use foundation::SyscallFrame;
use libc::{self, *};

use crate::handlers;

/// Upper bound for syscall numbers we will table-dispatch.
///
/// Linux uses `__NR_syscalls` as the syscall-space size (arch-dependent, typically a few hundred).
/// We pick a conservative bound to keep the table simple while staying small (~8 KiB on riscv64).
const NR_SYSCALLS: usize = 1024;

type SysHandler = fn(a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> isize;

#[inline(always)]
fn sys_unsupported_handler(
    _a0: usize,
    _a1: usize,
    _a2: usize,
    _a3: usize,
    _a4: usize,
    _a5: usize,
) -> isize {
    handlers::sys_unsupported()
}

macro_rules! sys_registry {
    (@call 0, $handler:path, $_a0:ident, $_a1:ident, $_a2:ident, $_a3:ident, $_a4:ident, $_a5:ident) => { $handler() };
    (@call 1, $handler:path, $_a0:ident, $_a1:ident, $_a2:ident, $_a3:ident, $_a4:ident, $_a5:ident) => { $handler($_a0) };
    (@call 2, $handler:path, $_a0:ident, $_a1:ident, $_a2:ident, $_a3:ident, $_a4:ident, $_a5:ident) => { $handler($_a0, $_a1) };
    (@call 3, $handler:path, $_a0:ident, $_a1:ident, $_a2:ident, $_a3:ident, $_a4:ident, $_a5:ident) => { $handler($_a0, $_a1, $_a2) };
    (@call 4, $handler:path, $_a0:ident, $_a1:ident, $_a2:ident, $_a3:ident, $_a4:ident, $_a5:ident) => { $handler($_a0, $_a1, $_a2, $_a3) };
    (@call 5, $handler:path, $_a0:ident, $_a1:ident, $_a2:ident, $_a3:ident, $_a4:ident, $_a5:ident) => { $handler($_a0, $_a1, $_a2, $_a3, $_a4) };
    (@call 6, $handler:path, $_a0:ident, $_a1:ident, $_a2:ident, $_a3:ident, $_a4:ident, $_a5:ident) => { $handler($_a0, $_a1, $_a2, $_a3, $_a4, $_a5) };

    // Emit module + handler wrapper for one syscall entry (plain).
    (@def $(#[$meta:meta])* ($nr:ident, $handler:path, $arity:tt)) => {
        $(#[$meta])*
        #[allow(non_snake_case)]
        mod $nr {
            use super::*;

            #[inline(always)]
            pub(super) fn h(
                _a0: usize,
                _a1: usize,
                _a2: usize,
                _a3: usize,
                _a4: usize,
                _a5: usize,
            ) -> isize {
                sys_registry!(@call $arity, $handler, _a0, _a1, _a2, _a3, _a4, _a5)
            }
        }
    };

    // Emit module + handler wrapper for one syscall entry (direct SysHandler).
    (@def $(#[$meta:meta])* ($nr:ident, $handler:path)) => {
        $(#[$meta])*
        #[allow(non_snake_case)]
        mod $nr {
            use super::*;

            #[inline(always)]
            pub(super) fn h(
                a0: usize,
                a1: usize,
                a2: usize,
                a3: usize,
                a4: usize,
                a5: usize,
            ) -> isize {
                $handler(a0, a1, a2, a3, a4, a5)
            }
        }
    };


    // Emit all wrapper modules (items at module scope).
    (@emit_defs ($($inh:tt)*) ) => {};
    (@emit_defs ($($inh:tt)*) , $($rest:tt)* ) => { sys_registry!(@emit_defs ($($inh)*) $($rest)*); };
    (@emit_defs ($($inh:tt)*) @pop_defs ($($old:tt)*) $($rest:tt)* ) => {
        sys_registry!(@emit_defs ($($old)*) $($rest)*);
    };
    (@emit_defs ($($inh:tt)*) $(#[$meta:meta])* { $($inner:tt)* } $($rest:tt)* ) => {
        sys_registry!(
            @emit_defs
            ($($inh)* $(#[$meta])*)
            $($inner)*
            @pop_defs ($($inh)*)
            $($rest)*
        );
    };
    (@emit_defs ($($inh:tt)*) $(#[$meta:meta])* ($nr:ident, $($rest_item:tt)+) $($rest:tt)* ) => {
        sys_registry!(@def $($inh)* $(#[$meta])* ($nr, $($rest_item)+));
        sys_registry!(@emit_defs ($($inh)*) $($rest)*);
    };

    // Emit table assignments (statements inside `build_handlers`, where `t` is in scope).
    (@emit_sets $t:ident ($($inh:tt)*) ) => {};
    (@emit_sets $t:ident ($($inh:tt)*) , $($rest:tt)* ) => { sys_registry!(@emit_sets $t ($($inh)*) $($rest)*); };
    (@emit_sets $t:ident ($($inh:tt)*) @pop_sets ($($old:tt)*) $($rest:tt)* ) => {
        sys_registry!(@emit_sets $t ($($old)*) $($rest)*);
    };
    (@emit_sets $t:ident ($($inh:tt)*) $(#[$meta:meta])* { $($inner:tt)* } $($rest:tt)* ) => {
        sys_registry!(
            @emit_sets
            $t
            ($($inh)* $(#[$meta])*)
            $($inner)*
            @pop_sets ($($inh)*)
            $($rest)*
        );
    };
    (@emit_sets $t:ident ($($inh:tt)*) $(#[$meta:meta])* ($nr:ident, $($rest_item:tt)+) $($rest:tt)* ) => {
        $($inh)* $(#[$meta])*
        {
            $t[$nr as usize] = $nr::h;
        }
        sys_registry!(@emit_sets $t ($($inh)*) $($rest)*);
    };

    // Entry point: emit defs + build the dense table.
    ( $($tokens:tt)* ) => {
        sys_registry!(@emit_defs () $($tokens)*);

        const fn build_handlers() -> [SysHandler; NR_SYSCALLS] {
            let mut t = [sys_unsupported_handler as SysHandler; NR_SYSCALLS];
            sys_registry!(@emit_sets t () $($tokens)*);
            t
        }

        static HANDLERS: [SysHandler; NR_SYSCALLS] = build_handlers();
    };
}

sys_registry! {
    // Always-supported syscalls (core process control).
    (SYS_exit, handlers::sys_exit, 1),
    (SYS_exit_group, handlers::sys_exit_group, 1),

    // Scheduler/sys-thread syscalls.
    #[cfg(feature = "scheduler")]
    {
        (SYS_clone, handlers::thread::sys_clone, 5),
        (SYS_futex, handlers::thread::sys_futex, 3),
        (SYS_sched_yield, handlers::thread::sys_sched_yield, 0),
        (SYS_getpid, handlers::thread::sys_getpid, 0),
        (SYS_gettid, handlers::thread::sys_gettid, 0),
        (SYS_set_tid_address, handlers::thread::sys_set_tid_address, 1),
    }

    // Memory syscalls.
    #[cfg(feature = "memory")]
    {
        (SYS_brk, handlers::memory::sys_brk, 1),
        (SYS_mmap, handlers::memory::sys_mmap, 6),
        (SYS_munmap, handlers::memory::sys_munmap, 2),
        (SYS_mprotect, handlers::memory::sys_mprotect, 3),
    }

    // VFS syscalls.
    #[cfg(feature = "vfs")]
    {
        (SYS_openat, handlers::vfs::sys_openat, 4),
        (SYS_close, handlers::vfs::sys_close, 1),
        (SYS_read, handlers::vfs::sys_read, 3),
        (SYS_write, handlers::vfs::sys_write, 3),
        (SYS_readv, handlers::vfs::sys_readv, 3),
        (SYS_writev, handlers::vfs::sys_writev, 3),
        (SYS_lseek, handlers::vfs::sys_lseek, 3),
        (SYS_ioctl, handlers::vfs::sys_ioctl, 3),
        (SYS_fstat, handlers::vfs::sys_fstat, 2),
    }

    // Random syscalls.
    #[cfg(feature = "random")]
    {
        (SYS_getrandom, handlers::random::sys_getrandom, 3),
    }
}

/// # Safety
/// `regs` must be a valid pointer to a syscall frame.
pub unsafe fn dispatch_syscall<Frame: SyscallFrame>(regs: *mut Frame) {
    let regs_ref = unsafe { &*regs };

    let nr = regs_ref.syscall_number();
    let a0 = regs_ref.arg(0);
    let a1 = regs_ref.arg(1);
    let a2 = regs_ref.arg(2);
    let a3 = regs_ref.arg(3);
    let a4 = regs_ref.arg(4);
    let a5 = regs_ref.arg(5);

    let ret = if nr < NR_SYSCALLS {
        (HANDLERS[nr])(a0, a1, a2, a3, a4, a5)
    } else {
        sys_unsupported_handler(a0, a1, a2, a3, a4, a5)
    };
    unsafe { (*regs).set_ret(ret) }
}

pub fn linux_handle(
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    nr: usize,
) -> isize {
    if nr < NR_SYSCALLS {
        (HANDLERS[nr])(a0, a1, a2, a3, a4, a5)
    } else {
        sys_unsupported_handler(a0, a1, a2, a3, a4, a5)
    }
}

pub const TRAP_OPS: foundation::ops::TrapOps = foundation::ops::TrapOps {
    syscall: linux_handle,
    exception: |_, _, _| None,
    interrupt: |_| {},
};
