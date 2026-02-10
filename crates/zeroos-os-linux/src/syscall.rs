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
    (SYS_rt_sigaction, handlers::signal::sys_rt_sigaction, 4),
    (SYS_rt_sigprocmask, handlers::signal::sys_rt_sigprocmask, 4),
    (SYS_tkill, handlers::signal::sys_tkill, 2),
    (SYS_tgkill, handlers::signal::sys_tgkill, 3),

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

/// Returns the name of a syscall given its number.
///
/// Only includes syscalls available on riscv64 (the primary target for zeroos-os-linux).
#[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
#[allow(non_snake_case)]
pub fn syscall_name(nr: usize) -> &'static str {
    match nr as i64 {
        // Process control
        SYS_exit => "SYS_exit",
        SYS_exit_group => "SYS_exit_group",
        SYS_clone => "SYS_clone",
        SYS_clone3 => "SYS_clone3",
        SYS_execve => "SYS_execve",
        SYS_execveat => "SYS_execveat",
        SYS_wait4 => "SYS_wait4",
        SYS_waitid => "SYS_waitid",
        SYS_kill => "SYS_kill",
        SYS_tkill => "SYS_tkill",
        SYS_tgkill => "SYS_tgkill",
        SYS_getpid => "SYS_getpid",
        SYS_getppid => "SYS_getppid",
        SYS_gettid => "SYS_gettid",
        SYS_set_tid_address => "SYS_set_tid_address",
        SYS_prctl => "SYS_prctl",
        SYS_ptrace => "SYS_ptrace",

        // Thread/futex
        SYS_futex => "SYS_futex",
        SYS_get_robust_list => "SYS_get_robust_list",
        SYS_set_robust_list => "SYS_set_robust_list",

        // Scheduling
        SYS_sched_yield => "SYS_sched_yield",
        SYS_sched_getaffinity => "SYS_sched_getaffinity",
        SYS_sched_setaffinity => "SYS_sched_setaffinity",
        SYS_sched_getparam => "SYS_sched_getparam",
        SYS_sched_setparam => "SYS_sched_setparam",
        SYS_sched_getscheduler => "SYS_sched_getscheduler",
        SYS_sched_setscheduler => "SYS_sched_setscheduler",
        SYS_sched_get_priority_max => "SYS_sched_get_priority_max",
        SYS_sched_get_priority_min => "SYS_sched_get_priority_min",
        SYS_sched_rr_get_interval => "SYS_sched_rr_get_interval",

        // Memory management
        SYS_brk => "SYS_brk",
        SYS_mmap => "SYS_mmap",
        SYS_munmap => "SYS_munmap",
        SYS_mprotect => "SYS_mprotect",
        SYS_mremap => "SYS_mremap",
        SYS_madvise => "SYS_madvise",
        SYS_mlock => "SYS_mlock",
        SYS_mlock2 => "SYS_mlock2",
        SYS_munlock => "SYS_munlock",
        SYS_mlockall => "SYS_mlockall",
        SYS_munlockall => "SYS_munlockall",
        SYS_msync => "SYS_msync",
        SYS_mincore => "SYS_mincore",
        SYS_membarrier => "SYS_membarrier",

        // File operations
        SYS_openat => "SYS_openat",
        SYS_openat2 => "SYS_openat2",
        SYS_close => "SYS_close",
        SYS_close_range => "SYS_close_range",
        SYS_read => "SYS_read",
        SYS_write => "SYS_write",
        SYS_readv => "SYS_readv",
        SYS_writev => "SYS_writev",
        SYS_pread64 => "SYS_pread64",
        SYS_pwrite64 => "SYS_pwrite64",
        SYS_preadv => "SYS_preadv",
        SYS_pwritev => "SYS_pwritev",
        SYS_preadv2 => "SYS_preadv2",
        SYS_pwritev2 => "SYS_pwritev2",
        SYS_lseek => "SYS_lseek",
        SYS_ioctl => "SYS_ioctl",
        SYS_fcntl => "SYS_fcntl",
        SYS_dup => "SYS_dup",
        SYS_dup3 => "SYS_dup3",
        SYS_flock => "SYS_flock",
        SYS_fsync => "SYS_fsync",
        SYS_fdatasync => "SYS_fdatasync",
        SYS_truncate => "SYS_truncate",
        SYS_ftruncate => "SYS_ftruncate",
        SYS_fallocate => "SYS_fallocate",
        SYS_fadvise64 => "SYS_fadvise64",
        SYS_readahead => "SYS_readahead",
        SYS_sendfile => "SYS_sendfile",
        SYS_splice => "SYS_splice",
        SYS_tee => "SYS_tee",
        SYS_sync_file_range => "SYS_sync_file_range",
        SYS_vmsplice => "SYS_vmsplice",
        SYS_copy_file_range => "SYS_copy_file_range",

        // File stat/metadata
        SYS_fstat => "SYS_fstat",
        SYS_newfstatat => "SYS_newfstatat",
        SYS_statx => "SYS_statx",
        SYS_faccessat => "SYS_faccessat",
        SYS_faccessat2 => "SYS_faccessat2",
        SYS_fchmod => "SYS_fchmod",
        SYS_fchmodat => "SYS_fchmodat",
        SYS_fchown => "SYS_fchown",
        SYS_fchownat => "SYS_fchownat",
        SYS_utimensat => "SYS_utimensat",

        // Directory operations
        SYS_getdents64 => "SYS_getdents64",
        SYS_getcwd => "SYS_getcwd",
        SYS_chdir => "SYS_chdir",
        SYS_fchdir => "SYS_fchdir",
        SYS_mkdirat => "SYS_mkdirat",
        SYS_mknodat => "SYS_mknodat",
        SYS_unlinkat => "SYS_unlinkat",
        SYS_renameat2 => "SYS_renameat2",
        SYS_linkat => "SYS_linkat",
        SYS_symlinkat => "SYS_symlinkat",
        SYS_readlinkat => "SYS_readlinkat",
        SYS_pivot_root => "SYS_pivot_root",
        SYS_mount => "SYS_mount",
        SYS_umount2 => "SYS_umount2",
        SYS_chroot => "SYS_chroot",

        // File descriptors / poll
        SYS_ppoll => "SYS_ppoll",
        SYS_pselect6 => "SYS_pselect6",
        SYS_epoll_create1 => "SYS_epoll_create1",
        SYS_epoll_ctl => "SYS_epoll_ctl",
        SYS_epoll_pwait => "SYS_epoll_pwait",
        SYS_epoll_pwait2 => "SYS_epoll_pwait2",
        SYS_eventfd2 => "SYS_eventfd2",
        SYS_signalfd4 => "SYS_signalfd4",
        SYS_timerfd_create => "SYS_timerfd_create",
        SYS_timerfd_settime => "SYS_timerfd_settime",
        SYS_timerfd_gettime => "SYS_timerfd_gettime",
        SYS_inotify_init1 => "SYS_inotify_init1",
        SYS_inotify_add_watch => "SYS_inotify_add_watch",
        SYS_inotify_rm_watch => "SYS_inotify_rm_watch",
        SYS_fanotify_init => "SYS_fanotify_init",
        SYS_fanotify_mark => "SYS_fanotify_mark",

        // Pipes
        SYS_pipe2 => "SYS_pipe2",

        // Sockets
        SYS_socket => "SYS_socket",
        SYS_socketpair => "SYS_socketpair",
        SYS_bind => "SYS_bind",
        SYS_listen => "SYS_listen",
        SYS_accept => "SYS_accept",
        SYS_accept4 => "SYS_accept4",
        SYS_connect => "SYS_connect",
        SYS_getsockname => "SYS_getsockname",
        SYS_getpeername => "SYS_getpeername",
        SYS_sendto => "SYS_sendto",
        SYS_recvfrom => "SYS_recvfrom",
        SYS_sendmsg => "SYS_sendmsg",
        SYS_recvmsg => "SYS_recvmsg",
        SYS_sendmmsg => "SYS_sendmmsg",
        SYS_recvmmsg => "SYS_recvmmsg",
        SYS_shutdown => "SYS_shutdown",
        SYS_setsockopt => "SYS_setsockopt",
        SYS_getsockopt => "SYS_getsockopt",

        // Signals
        SYS_rt_sigaction => "SYS_rt_sigaction",
        SYS_rt_sigprocmask => "SYS_rt_sigprocmask",
        SYS_rt_sigreturn => "SYS_rt_sigreturn",
        SYS_rt_sigsuspend => "SYS_rt_sigsuspend",
        SYS_rt_sigpending => "SYS_rt_sigpending",
        SYS_rt_sigtimedwait => "SYS_rt_sigtimedwait",
        SYS_rt_sigqueueinfo => "SYS_rt_sigqueueinfo",
        SYS_rt_tgsigqueueinfo => "SYS_rt_tgsigqueueinfo",
        SYS_sigaltstack => "SYS_sigaltstack",

        // Time
        SYS_clock_gettime => "SYS_clock_gettime",
        SYS_clock_settime => "SYS_clock_settime",
        SYS_clock_getres => "SYS_clock_getres",
        SYS_clock_nanosleep => "SYS_clock_nanosleep",
        SYS_clock_adjtime => "SYS_clock_adjtime",
        SYS_gettimeofday => "SYS_gettimeofday",
        SYS_settimeofday => "SYS_settimeofday",
        SYS_adjtimex => "SYS_adjtimex",
        SYS_nanosleep => "SYS_nanosleep",
        SYS_getitimer => "SYS_getitimer",
        SYS_setitimer => "SYS_setitimer",
        SYS_times => "SYS_times",
        SYS_timer_create => "SYS_timer_create",
        SYS_timer_settime => "SYS_timer_settime",
        SYS_timer_gettime => "SYS_timer_gettime",
        SYS_timer_getoverrun => "SYS_timer_getoverrun",
        SYS_timer_delete => "SYS_timer_delete",

        // User/group IDs
        SYS_getuid => "SYS_getuid",
        SYS_geteuid => "SYS_geteuid",
        SYS_getgid => "SYS_getgid",
        SYS_getegid => "SYS_getegid",
        SYS_setuid => "SYS_setuid",
        SYS_setgid => "SYS_setgid",
        SYS_setreuid => "SYS_setreuid",
        SYS_setregid => "SYS_setregid",
        SYS_setresuid => "SYS_setresuid",
        SYS_setresgid => "SYS_setresgid",
        SYS_getresuid => "SYS_getresuid",
        SYS_getresgid => "SYS_getresgid",
        SYS_setfsuid => "SYS_setfsuid",
        SYS_setfsgid => "SYS_setfsgid",
        SYS_getgroups => "SYS_getgroups",
        SYS_setgroups => "SYS_setgroups",

        // Session/process group
        SYS_setsid => "SYS_setsid",
        SYS_getsid => "SYS_getsid",
        SYS_setpgid => "SYS_setpgid",
        SYS_getpgid => "SYS_getpgid",
        SYS_vhangup => "SYS_vhangup",

        // Resource limits
        SYS_getrlimit => "SYS_getrlimit",
        SYS_setrlimit => "SYS_setrlimit",
        SYS_prlimit64 => "SYS_prlimit64",
        SYS_getrusage => "SYS_getrusage",
        SYS_getpriority => "SYS_getpriority",
        SYS_setpriority => "SYS_setpriority",

        // System info
        SYS_uname => "SYS_uname",
        SYS_sysinfo => "SYS_sysinfo",
        SYS_syslog => "SYS_syslog",
        SYS_getrandom => "SYS_getrandom",
        SYS_getcpu => "SYS_getcpu",

        // Capabilities
        SYS_capget => "SYS_capget",
        SYS_capset => "SYS_capset",

        // Misc
        SYS_umask => "SYS_umask",
        SYS_personality => "SYS_personality",
        SYS_reboot => "SYS_reboot",
        SYS_sync => "SYS_sync",
        SYS_syncfs => "SYS_syncfs",
        SYS_statfs => "SYS_statfs",
        SYS_fstatfs => "SYS_fstatfs",
        SYS_swapon => "SYS_swapon",
        SYS_swapoff => "SYS_swapoff",
        SYS_sethostname => "SYS_sethostname",
        SYS_setdomainname => "SYS_setdomainname",
        SYS_acct => "SYS_acct",
        SYS_quotactl => "SYS_quotactl",
        SYS_nfsservctl => "SYS_nfsservctl",
        SYS_lookup_dcookie => "SYS_lookup_dcookie",
        SYS_remap_file_pages => "SYS_remap_file_pages",
        SYS_restart_syscall => "SYS_restart_syscall",

        // Modules
        SYS_init_module => "SYS_init_module",
        SYS_delete_module => "SYS_delete_module",
        SYS_finit_module => "SYS_finit_module",
        SYS_kexec_load => "SYS_kexec_load",

        // io_uring
        SYS_io_uring_setup => "SYS_io_uring_setup",
        SYS_io_uring_enter => "SYS_io_uring_enter",
        SYS_io_uring_register => "SYS_io_uring_register",

        // Async I/O (legacy)
        SYS_io_setup => "SYS_io_setup",
        SYS_io_destroy => "SYS_io_destroy",
        SYS_io_submit => "SYS_io_submit",
        SYS_io_cancel => "SYS_io_cancel",
        SYS_io_getevents => "SYS_io_getevents",
        SYS_io_pgetevents => "SYS_io_pgetevents",

        // Shared memory
        SYS_shmget => "SYS_shmget",
        SYS_shmat => "SYS_shmat",
        SYS_shmdt => "SYS_shmdt",
        SYS_shmctl => "SYS_shmctl",

        // Semaphores
        SYS_semget => "SYS_semget",
        SYS_semop => "SYS_semop",
        SYS_semtimedop => "SYS_semtimedop",
        SYS_semctl => "SYS_semctl",

        // Message queues
        SYS_msgget => "SYS_msgget",
        SYS_msgsnd => "SYS_msgsnd",
        SYS_msgrcv => "SYS_msgrcv",
        SYS_msgctl => "SYS_msgctl",
        SYS_mq_open => "SYS_mq_open",
        SYS_mq_unlink => "SYS_mq_unlink",
        SYS_mq_timedsend => "SYS_mq_timedsend",
        SYS_mq_timedreceive => "SYS_mq_timedreceive",
        SYS_mq_notify => "SYS_mq_notify",
        SYS_mq_getsetattr => "SYS_mq_getsetattr",

        // Keys
        SYS_add_key => "SYS_add_key",
        SYS_request_key => "SYS_request_key",
        SYS_keyctl => "SYS_keyctl",

        // Extended attributes
        SYS_setxattr => "SYS_setxattr",
        SYS_lsetxattr => "SYS_lsetxattr",
        SYS_fsetxattr => "SYS_fsetxattr",
        SYS_getxattr => "SYS_getxattr",
        SYS_lgetxattr => "SYS_lgetxattr",
        SYS_fgetxattr => "SYS_fgetxattr",
        SYS_listxattr => "SYS_listxattr",
        SYS_llistxattr => "SYS_llistxattr",
        SYS_flistxattr => "SYS_flistxattr",
        SYS_removexattr => "SYS_removexattr",
        SYS_lremovexattr => "SYS_lremovexattr",
        SYS_fremovexattr => "SYS_fremovexattr",

        // Namespaces
        SYS_setns => "SYS_setns",
        SYS_unshare => "SYS_unshare",

        // NUMA
        SYS_mbind => "SYS_mbind",
        SYS_set_mempolicy => "SYS_set_mempolicy",
        SYS_get_mempolicy => "SYS_get_mempolicy",
        SYS_migrate_pages => "SYS_migrate_pages",
        SYS_move_pages => "SYS_move_pages",

        // Priority I/O
        SYS_ioprio_set => "SYS_ioprio_set",
        SYS_ioprio_get => "SYS_ioprio_get",

        // Process VM
        SYS_process_vm_readv => "SYS_process_vm_readv",
        SYS_process_vm_writev => "SYS_process_vm_writev",
        SYS_process_madvise => "SYS_process_madvise",
        SYS_kcmp => "SYS_kcmp",

        // Scheduling (newer)
        SYS_sched_setattr => "SYS_sched_setattr",
        SYS_sched_getattr => "SYS_sched_getattr",

        // Memory (newer)
        SYS_memfd_create => "SYS_memfd_create",
        SYS_pkey_mprotect => "SYS_pkey_mprotect",
        SYS_pkey_alloc => "SYS_pkey_alloc",
        SYS_pkey_free => "SYS_pkey_free",
        SYS_rseq => "SYS_rseq",

        // Mount (newer)
        SYS_open_tree => "SYS_open_tree",
        SYS_move_mount => "SYS_move_mount",
        SYS_fsopen => "SYS_fsopen",
        SYS_fsconfig => "SYS_fsconfig",
        SYS_fsmount => "SYS_fsmount",
        SYS_fspick => "SYS_fspick",
        SYS_mount_setattr => "SYS_mount_setattr",
        SYS_name_to_handle_at => "SYS_name_to_handle_at",
        SYS_open_by_handle_at => "SYS_open_by_handle_at",

        // Security
        SYS_seccomp => "SYS_seccomp",
        SYS_bpf => "SYS_bpf",
        SYS_landlock_create_ruleset => "SYS_landlock_create_ruleset",
        SYS_landlock_add_rule => "SYS_landlock_add_rule",
        SYS_landlock_restrict_self => "SYS_landlock_restrict_self",

        // Misc newer syscalls
        SYS_userfaultfd => "SYS_userfaultfd",
        SYS_perf_event_open => "SYS_perf_event_open",
        SYS_pidfd_open => "SYS_pidfd_open",
        SYS_pidfd_send_signal => "SYS_pidfd_send_signal",
        SYS_pidfd_getfd => "SYS_pidfd_getfd",

        _ => "SYS_unknown",
    }
}

/// Fallback for non-riscv targets (for cross-compilation).
#[cfg(not(any(target_arch = "riscv64", target_arch = "riscv32")))]
pub fn syscall_name(_nr: usize) -> &'static str {
    "SYS_unknown"
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
