//! Architecture-specific operation table.
//!
//! This table abstracts low-level CPU operations like context switching,
//! trap frame initialization, and stack management. Schedulers and OS
//! personalities use these operations to remain architecture-agnostic.

#[derive(Clone, Copy)]
pub struct ArchOps {
    /// Return the size (bytes) of the arch thread context structure (for context switching).
    pub thread_ctx_size: fn() -> usize,
    /// Return the alignment (bytes) of the arch thread context structure.
    pub thread_ctx_align: fn() -> usize,

    /// Return the size (bytes) of the arch TrapFrame structure (for traps/syscalls).
    pub trap_frame_size: fn() -> usize,
    /// Return the alignment (bytes) of the arch TrapFrame structure.
    pub trap_frame_align: fn() -> usize,

    /// Initialize a newly allocated arch thread context.
    /// # Safety
    /// `ctx_ptr` must be a valid, aligned, and mutable pointer.
    pub thread_ctx_init: unsafe fn(ctx_ptr: *mut u8, anchor: usize, kstack_top: usize),
    /// # Safety
    /// `ctx_ptr` must be a valid, aligned, and mutable pointer.
    pub thread_ctx_set_sp: unsafe fn(ctx_ptr: *mut u8, sp: usize),
    /// # Safety
    /// `ctx_ptr` must be a valid, aligned, and mutable pointer.
    pub thread_ctx_set_tp: unsafe fn(ctx_ptr: *mut u8, tp: usize),
    /// # Safety
    /// `ctx_ptr` must be a valid, aligned, and mutable pointer.
    pub thread_ctx_set_ra: unsafe fn(ctx_ptr: *mut u8, ra: usize),
    /// # Safety
    /// `ctx_ptr` must be a valid, aligned, and mutable pointer.
    pub thread_ctx_set_retval: unsafe fn(ctx_ptr: *mut u8, val: usize),

    /// Switch CPU execution from `old` to `new` (callee-saved context switch).
    /// # Safety
    /// Both pointers must be valid, aligned, and readable/writable.
    pub switch_to: unsafe extern "C" fn(old_ctx_ptr: *mut u8, new_ctx_ptr: *const u8),

    /// Return the address of the arch-specific assembly trampoline used to
    /// restore a TrapFrame and execute a return-from-trap instruction (e.g., mret).
    ///
    /// This is used by the scheduler to "jump-start" a newly spawned thread:
    /// the scheduler sets the thread's initial kernel return address (`ra`) to this
    /// trampoline, and the return value (`retval`) to the address of the TrapFrame.
    pub ret_from_fork: fn() -> usize,

    /// TrapFrame operations (treat it as an opaque blob to scheduler code).
    /// # Safety
    /// `dst` and `src` must be valid, aligned pointers.
    pub trap_frame_clone: unsafe fn(dst: *mut u8, src: *const u8),
    /// # Safety
    /// `regs` must be a valid, aligned, and mutable pointer.
    pub trap_frame_init: unsafe fn(regs: *mut u8, user_sp: usize, user_tls: usize, pc: usize),
    /// # Safety
    /// `regs` must be a valid, aligned, and mutable pointer.
    pub trap_frame_set_retval: unsafe fn(regs: *mut u8, val: usize),
    /// # Safety
    /// `regs` must be a valid, aligned, and mutable pointer.
    pub trap_frame_set_sp: unsafe fn(regs: *mut u8, sp: usize),
    /// # Safety
    /// `regs` must be a valid, aligned, and mutable pointer.
    pub trap_frame_set_tp: unsafe fn(regs: *mut u8, tp: usize),

    /// Return a pointer to the current CPU's trap frame.
    ///
    /// # Safety
    /// Must be called from a context where `tp` points to a valid `ThreadAnchor`.
    pub current_trap_frame: unsafe fn() -> *mut u8,

    /// Return the saved resume PC from a trap frame.
    /// # Safety
    /// `regs` must be a valid, aligned pointer.
    pub trap_frame_get_pc: unsafe fn(regs: *const u8) -> usize,
    /// Set the resume PC in a trap frame.
    /// # Safety
    /// `regs` must be a valid, aligned pointer.
    pub trap_frame_set_pc: unsafe fn(regs: *mut u8, pc: usize),

    /// Return the architecture-specific syscall number from a trap frame.
    /// # Safety
    /// `regs` must be a valid, aligned pointer.
    pub trap_frame_get_nr: unsafe fn(regs: *const u8) -> usize,
    /// Return the syscall argument at the given index (0-5) from a trap frame.
    /// # Safety
    /// `regs` must be a valid, aligned pointer.
    pub trap_frame_get_arg: unsafe fn(regs: *const u8, idx: usize) -> usize,

    /// Return the trap cause/code from a trap frame.
    /// # Safety
    /// `regs` must be a valid, aligned pointer.
    pub trap_frame_get_cause: unsafe fn(regs: *const u8) -> usize,
    /// Return the trap value (e.g. faulting address) from a trap frame.
    /// # Safety
    /// `regs` must be a valid, aligned pointer.
    pub trap_frame_get_fault_addr: unsafe fn(regs: *const u8) -> usize,
}
