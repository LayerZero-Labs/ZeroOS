//! Arch-specific per-thread switch context.

use core::mem::{align_of, size_of};

/// Must match the save/restore order in `switch_to` asm.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ThreadContext {
    pub sp: usize,
    pub tp: usize,
    pub ra: usize,
    pub gp: usize,
    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub retval: usize,
}

impl Default for ThreadContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadContext {
    #[inline]
    pub const fn new() -> Self {
        Self {
            sp: 0,
            tp: 0,
            ra: 0,
            gp: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
            retval: 0,
        }
    }
}

#[inline]
pub fn thread_ctx_size() -> usize {
    size_of::<ThreadContext>()
}

#[inline]
pub fn thread_ctx_align() -> usize {
    align_of::<ThreadContext>()
}

#[inline]
/// # Safety
/// `p` must point to a valid `ThreadContext` structure.
unsafe fn ctx_mut(p: *mut u8) -> &'static mut ThreadContext {
    &mut *(p as *mut ThreadContext)
}

/// # Safety
/// `ctx_ptr` must point to a valid, aligned region of at least `thread_ctx_size()` bytes.
pub unsafe fn thread_ctx_init(ctx_ptr: *mut u8, anchor: usize, kstack_top: usize) {
    core::ptr::write(ctx_ptr as *mut ThreadContext, ThreadContext::new());
    let c = ctx_mut(ctx_ptr);
    c.tp = anchor;
    c.sp = kstack_top;

    // Capture the current global pointer so the new thread starts with the correct gp.
    let current_gp: usize;
    core::arch::asm!("mv {}, gp", out(reg) current_gp);
    c.gp = current_gp;
}

/// # Safety
/// `ctx_ptr` must point to a valid `ThreadContext` structure.
pub unsafe fn thread_ctx_set_sp(ctx_ptr: *mut u8, sp: usize) {
    ctx_mut(ctx_ptr).sp = sp;
}

/// # Safety
/// `ctx_ptr` must point to a valid `ThreadContext` structure.
pub unsafe fn thread_ctx_set_tp(ctx_ptr: *mut u8, tp: usize) {
    ctx_mut(ctx_ptr).tp = tp;
}

/// # Safety
/// `ctx_ptr` must point to a valid `ThreadContext` structure.
pub unsafe fn thread_ctx_set_ra(ctx_ptr: *mut u8, ra: usize) {
    ctx_mut(ctx_ptr).ra = ra;
}

/// # Safety
/// `ctx_ptr` must point to a valid `ThreadContext` structure.
pub unsafe fn thread_ctx_set_retval(ctx_ptr: *mut u8, val: usize) {
    ctx_mut(ctx_ptr).retval = val;
}
