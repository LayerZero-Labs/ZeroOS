//! Common interface for architecture-specific register state.
//!
//! This module defines traits for manipulating CPU contexts and syscall frames, used
//! by the scheduler and syscall handlers.

pub trait SyscallFrame: Sized {
    /// Return the saved program counter.
    fn pc(&self) -> usize;
    /// Return the architecture-specific syscall number.
    fn syscall_number(&self) -> usize;
    /// Return the syscall argument at the given index (0-5).
    fn arg(&self, idx: usize) -> usize;
    /// Set the syscall return value.
    fn set_ret(&mut self, ret: isize);
}
