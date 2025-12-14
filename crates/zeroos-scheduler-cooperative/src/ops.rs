use crate::context::Context;
use crate::scheduler::Scheduler;
use foundation::ArchContext;
use libc;

pub fn init() {
    Scheduler::init();
}

pub fn spawn_thread(
    stack: usize,
    tls: usize,
    parent_tid_ptr: usize,
    child_tid_ptr: usize,
    clear_child_tid_ptr: usize,
    mepc: usize,
    frame_ptr: usize,
) -> isize {
    Scheduler::with_mut(|scheduler| {
        let parent_context = if frame_ptr != 0 {
            unsafe { core::ptr::read(frame_ptr as *const Context) }
        } else if let Some(tcb) = scheduler.current_thread() {
            unsafe { (*tcb.as_ptr()).trap_frame }
        } else {
            return -(libc::EPERM as isize);
        };

        let tid = scheduler.spawn_thread(parent_context, stack, tls, clear_child_tid_ptr, mepc);

        if tid > 0 {
            let tid = tid as i32;
            unsafe {
                if parent_tid_ptr != 0 {
                    (parent_tid_ptr as *mut i32).write_volatile(tid);
                }
                if child_tid_ptr != 0 {
                    (child_tid_ptr as *mut i32).write_volatile(tid);
                }
            }
            if frame_ptr != 0 {
                unsafe {
                    let frame = &mut *(frame_ptr as *mut Context);
                    frame.set_return_value(tid as usize);
                }
            }
        }

        tid
    })
    .unwrap_or(-(libc::EPERM as isize))
}

pub fn yield_now() -> isize {
    Scheduler::with_mut(|scheduler| scheduler.yield_now());
    0
}

pub fn exit_current(code: i32) -> isize {
    Scheduler::with_mut(|scheduler| scheduler.exit_current_and_yield(code)).unwrap_or_else(|| {
        extern "C" {
            static mut tohost: u64;
        }
        unsafe {
            let payload = ((code as u64) << 1) | 1;
            core::ptr::write_volatile(&raw mut tohost, payload);
        }
        0
    })
}

pub fn current_tid() -> usize {
    Scheduler::with_mut(|scheduler| scheduler.current_tid_or_1()).unwrap_or(1)
}

#[inline(always)]
pub fn thread_count() -> usize {
    Scheduler::with_mut(|s| s.thread_count()).unwrap_or(1)
}

#[inline(always)]
pub fn wait_on_addr(addr: usize, val: i32) -> isize {
    Scheduler::with_mut(|scheduler| scheduler.wait_on_addr(addr, val)).unwrap_or(0)
}

#[inline(always)]
pub fn wake_on_addr(addr: usize, count: usize) -> usize {
    Scheduler::with_mut(|scheduler| scheduler.wake_on_addr(addr, count)).unwrap_or(0)
}

pub fn set_tid_address(tidptr: usize) -> isize {
    Scheduler::with_mut(|scheduler| {
        if let Some(tcb) = scheduler.current_thread() {
            unsafe {
                (*tcb.as_ptr()).clear_child_tid = tidptr;
                (*tcb.as_ptr()).tid as isize
            }
        } else {
            0
        }
    })
    .unwrap_or(0)
}

pub const SCHEDULER_OPS: foundation::ops::SchedulerOps = foundation::ops::SchedulerOps {
    init,
    spawn_thread,
    yield_now,
    exit_current,
    current_tid,
    thread_count,
    wait_on_addr,
    wake_on_addr,
    set_clear_on_exit_addr: set_tid_address,
    update_frame: crate::trap_glue::update_frame,
    finish_trap: crate::trap_glue::finish_trap,
};
