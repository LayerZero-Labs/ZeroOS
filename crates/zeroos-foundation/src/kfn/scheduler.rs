#[inline]
pub fn kinit() {
    unsafe { (crate::KERNEL.scheduler.init)() }
}

#[inline]
pub fn spawn_thread(
    stack: usize,
    tls: usize,
    parent_tid_ptr: usize,
    child_tid_ptr: usize,
    clear_child_tid_ptr: usize,
    pc: usize,
    frame_ptr: usize,
) -> isize {
    unsafe {
        (crate::KERNEL.scheduler.spawn_thread)(
            stack,
            tls,
            parent_tid_ptr,
            child_tid_ptr,
            clear_child_tid_ptr,
            pc,
            frame_ptr,
        )
    }
}

#[inline]
pub fn sched_yield() -> isize {
    unsafe { (crate::KERNEL.scheduler.yield_now)() }
}

#[inline]
pub fn exit_current(code: i32) -> isize {
    unsafe { (crate::KERNEL.scheduler.exit_current)(code) }
}

#[inline]
pub fn current_tid() -> usize {
    unsafe { (crate::KERNEL.scheduler.current_tid)() }
}

#[inline]
pub fn thread_count() -> usize {
    unsafe { (crate::KERNEL.scheduler.thread_count)() }
}

#[inline]
pub fn wait_on_addr(addr: usize, expected: i32) -> isize {
    unsafe { (crate::KERNEL.scheduler.wait_on_addr)(addr, expected) }
}
#[inline]
pub fn wake_on_addr(addr: usize, count: usize) -> usize {
    unsafe { (crate::KERNEL.scheduler.wake_on_addr)(addr, count) }
}

#[inline]
pub fn set_clear_on_exit_addr(addr: usize) -> isize {
    unsafe { (crate::KERNEL.scheduler.set_clear_on_exit_addr)(addr) }
}

#[inline]
pub fn update_frame(frame_ptr: usize, pc: usize) {
    unsafe { (crate::KERNEL.scheduler.update_frame)(frame_ptr, pc) }
}
#[inline]
pub fn finish_trap(frame_ptr: usize, pc_ptr: usize, pc: usize) {
    unsafe { (crate::KERNEL.scheduler.finish_trap)(frame_ptr, pc_ptr, pc) }
}
