#[derive(Clone, Copy)]
pub struct SchedulerOps {
    pub init: fn(),
    pub spawn_thread: fn(
        stack: usize,
        tls: usize,
        parent_tid_ptr: usize,
        child_tid_ptr: usize,
        clear_child_tid_ptr: usize,
        pc: usize,
        frame_ptr: usize,
    ) -> isize,
    pub yield_now: fn() -> isize,
    pub exit_current: fn(code: i32) -> isize,

    pub current_tid: fn() -> usize,
    pub thread_count: fn() -> usize,

    pub wait_on_addr: fn(addr: usize, expected: i32) -> isize,
    pub wake_on_addr: fn(addr: usize, count: usize) -> usize,
    pub set_clear_on_exit_addr: fn(addr: usize) -> isize,

    pub update_frame: fn(frame_ptr: usize, pc: usize),
    pub finish_trap: fn(frame_ptr: usize, pc_ptr: usize, pc: usize),
}
