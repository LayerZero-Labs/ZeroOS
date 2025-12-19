//! Scheduler operation table.
//!
//! Defines the interface for a thread scheduler.

#[derive(Clone, Copy)]
pub struct SchedulerOps {
    /// Initialize the scheduler and create the boot thread.
    /// Returns the `ThreadAnchor` address for the boot thread.
    pub init: fn() -> usize,

    /// Spawn a new thread with the given stack, TLS, and TIDs pointers.
    pub spawn_thread: fn(
        stack: usize,
        tls: usize,
        parent_tid_ptr: usize,
        child_tid_ptr: usize,
        clear_child_tid_ptr: usize,
    ) -> isize,

    /// Voluntarily yield the CPU to another thread.
    pub yield_now: fn() -> isize,

    /// Terminate the current thread with the given exit code.
    pub exit_current: fn(code: i32) -> isize,

    /// Return the TID of the current thread.
    pub current_tid: fn() -> usize,

    /// Return the total number of managed threads.
    pub thread_count: fn() -> usize,

    /// Put the current thread to sleep until the value at `addr` changes.
    pub wait_on_addr: fn(addr: usize, expected: i32) -> isize,

    /// Wake up to `count` threads waiting on `addr`.
    pub wake_on_addr: fn(addr: usize, count: usize) -> usize,

    /// Set a memory address to be cleared when the current thread exits.
    pub set_clear_on_exit_addr: fn(addr: usize) -> isize,
}
