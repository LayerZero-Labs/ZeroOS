//! Trap operation table.
//!
//! Defines the interface for handling different types of CPU traps.

#[derive(Clone, Copy)]
pub struct TrapOps {
    /// Handle a system call.
    pub syscall:
        fn(a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, nr: usize) -> isize,

    /// Handle a CPU exception (fault).
    /// Returns `Some(new_pc)` to resume, or `None` to terminate.
    pub exception: fn(code: usize, pc: usize, trap_value: usize) -> Option<usize>,

    /// Handle a hardware interrupt.
    pub interrupt: fn(code: usize),
}

impl Default for TrapOps {
    fn default() -> Self {
        Self {
            syscall: |_, _, _, _, _, _, _| -38, // ENOSYS
            exception: |_, _, _| None,
            interrupt: |_| {},
        }
    }
}
