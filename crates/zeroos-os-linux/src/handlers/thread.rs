use libc;

use foundation::kfn;

pub fn sys_clone(
    flags: usize,
    stack: usize,
    parent_tid: usize,
    tls: usize,
    child_tid: usize,
) -> isize {
    if stack == 0 {
        return -(libc::EINVAL as isize);
    }
    if parent_tid != 0 && !parent_tid.is_multiple_of(core::mem::align_of::<i32>()) {
        return -(libc::EINVAL as isize);
    }
    if child_tid != 0 && !child_tid.is_multiple_of(core::mem::align_of::<i32>()) {
        return -(libc::EINVAL as isize);
    }

    // Linux `clone(2)` packs a signal number in the low 8 bits.
    // We allow any signal value, but reject unsupported flag bits.
    let allowed_flags = (libc::CLONE_VM
        | libc::CLONE_FS
        | libc::CLONE_FILES
        | libc::CLONE_SIGHAND
        | libc::CLONE_SYSVSEM
        | libc::CLONE_THREAD
        | libc::CLONE_SETTLS
        | libc::CLONE_PARENT_SETTID
        | libc::CLONE_CHILD_CLEARTID
        | libc::CLONE_CHILD_SETTID
        | libc::CLONE_DETACHED) as usize;
    let flags_no_sig = flags & !0xff;
    if (flags_no_sig & !allowed_flags) != 0 {
        return -(libc::EINVAL as isize);
    }
    if (flags & libc::CLONE_PARENT_SETTID as usize) != 0 && parent_tid == 0 {
        return -(libc::EINVAL as isize);
    }
    if (flags & libc::CLONE_CHILD_CLEARTID as usize) != 0 && child_tid == 0 {
        return -(libc::EINVAL as isize);
    }
    if (flags & libc::CLONE_CHILD_SETTID as usize) != 0 && child_tid == 0 {
        return -(libc::EINVAL as isize);
    }
    if (flags & libc::CLONE_SETTLS as usize) != 0 && tls == 0 {
        return -(libc::EINVAL as isize);
    }

    let tls_val = if (flags & libc::CLONE_SETTLS as usize) != 0 {
        tls
    } else {
        0
    };
    let parent_tid_ptr = if (flags & libc::CLONE_PARENT_SETTID as usize) != 0 {
        parent_tid
    } else {
        0
    };
    let child_tid_ptr =
        if (flags & (libc::CLONE_CHILD_SETTID as usize | libc::CLONE_CHILD_CLEARTID as usize)) != 0
        {
            child_tid
        } else {
            0
        };
    let clear_child_tid_ptr = if (flags & libc::CLONE_CHILD_CLEARTID as usize) != 0 {
        child_tid
    } else {
        0
    };

    kfn::scheduler::kspawn_thread(
        stack,
        tls_val,
        parent_tid_ptr,
        child_tid_ptr,
        clear_child_tid_ptr,
    )
}

pub fn sys_exit(status: usize) -> isize {
    kfn::scheduler::kexit_current(status as i32)
}

pub fn sys_exit_group(status: usize) -> isize {
    kfn::scheduler::kexit_current(status as i32)
}

pub fn sys_futex(addr: usize, op: usize, val: usize) -> isize {
    if addr == 0 || !addr.is_multiple_of(core::mem::align_of::<i32>()) {
        return -(libc::EINVAL as isize);
    }
    let op_i32 = op as i32;
    let cmd = op_i32 & libc::FUTEX_CMD_MASK;

    match cmd {
        libc::FUTEX_WAIT | libc::FUTEX_WAIT_BITSET => {
            kfn::scheduler::kwait_on_addr(addr, val as i32)
        }

        libc::FUTEX_WAKE | libc::FUTEX_WAKE_BITSET => {
            kfn::scheduler::kwake_on_addr(addr, val) as isize
        }
        _ => -(libc::EINVAL as isize),
    }
}

pub fn sys_sched_yield() -> isize {
    kfn::scheduler::ksched_yield()
}

pub fn sys_getpid() -> isize {
    1
}

pub fn sys_gettid() -> isize {
    kfn::scheduler::kcurrent_tid() as isize
}

pub fn sys_set_tid_address(tidptr: usize) -> isize {
    if tidptr != 0 && !tidptr.is_multiple_of(core::mem::align_of::<i32>()) {
        return -(libc::EINVAL as isize);
    }
    kfn::scheduler::kset_clear_on_exit_addr(tidptr)
}
