//! `foundation::ops::ArchOps` implementation for RISC-V.

use core::mem::size_of;

use foundation::ops::ArchOps;

use crate::ret_from_fork::ret_from_fork;
use crate::switch_to::switch_to;
use crate::trap::TrapFrame;
use foundation::kfn::thread::ThreadAnchor;

/// # Safety
/// `dst` and `src` must point to valid `TrapFrame` memory regions.
unsafe fn trap_frame_clone(dst: *mut u8, src: *const u8) {
    core::ptr::copy_nonoverlapping(src, dst, size_of::<TrapFrame>());
}

/// # Safety
/// `regs` must point to a valid, aligned region of at least `size_of::<TrapFrame>()` bytes.
unsafe fn trap_frame_init(regs: *mut u8, user_sp: usize, user_tls: usize, pc: usize) {
    let r = &mut *(regs as *mut TrapFrame);
    *r = TrapFrame::new();
    r.sp = user_sp;
    r.tp = user_tls;
    r.mepc = pc;
    // New thread returns 0 from clone in child context.
    r.a0 = 0;
}

/// # Safety
/// `regs` must point to a valid `TrapFrame`.
#[inline(always)]
unsafe fn trap_frame_set_retval(regs: *mut u8, val: usize) {
    let r = &mut *(regs as *mut TrapFrame);
    r.a0 = val;
}

/// # Safety
/// `regs` must point to a valid `TrapFrame`.
#[inline(always)]
unsafe fn trap_frame_set_sp(regs: *mut u8, sp: usize) {
    let r = &mut *(regs as *mut TrapFrame);
    r.sp = sp;
}

/// # Safety
/// `regs` must point to a valid `TrapFrame`.
#[inline(always)]
unsafe fn trap_frame_set_tp(regs: *mut u8, tp: usize) {
    let r = &mut *(regs as *mut TrapFrame);
    r.tp = tp;
}

/// # Safety
/// Must be called when `tp` contains a valid `ThreadAnchor` pointer.
#[inline(always)]
unsafe fn current_trap_frame() -> *mut u8 {
    // Kernel `tp` always points to the current `ThreadAnchor` in our trap/scheduler convention.
    let tp: usize;
    core::arch::asm!("mv {0}, tp", out(reg) tp, options(nomem, nostack, preserves_flags));
    let anchor = tp as *const ThreadAnchor;
    let regs_addr = foundation::kfn::thread::ktrap_frame_addr(anchor);
    regs_addr as *mut u8
}

/// # Safety
/// `regs` must point to a valid `TrapFrame`.
#[inline(always)]
unsafe fn trap_frame_get_pc(regs: *const u8) -> usize {
    let r = &*(regs as *const TrapFrame);
    r.mepc
}

/// # Safety
/// `regs` must point to a valid `TrapFrame`.
#[inline(always)]
unsafe fn trap_frame_set_pc(regs: *mut u8, pc: usize) {
    let r = &mut *(regs as *mut TrapFrame);
    r.mepc = pc;
}

/// # Safety
/// `regs` must point to a valid `TrapFrame`.
#[inline(always)]
unsafe fn trap_frame_get_nr(regs: *const u8) -> usize {
    let r = &*(regs as *const TrapFrame);
    r.a7
}

/// # Safety
/// `regs` must point to a valid `TrapFrame`.
#[inline(always)]
unsafe fn trap_frame_get_arg(regs: *const u8, idx: usize) -> usize {
    let r = &*(regs as *const TrapFrame);
    match idx {
        0 => r.a0,
        1 => r.a1,
        2 => r.a2,
        3 => r.a3,
        4 => r.a4,
        5 => r.a5,
        _ => 0,
    }
}

/// # Safety
/// `regs` must point to a valid `TrapFrame`.
#[inline(always)]
unsafe fn trap_frame_get_cause(regs: *const u8) -> usize {
    let r = &*(regs as *const TrapFrame);
    r.mcause
}

/// # Safety
/// `regs` must point to a valid `TrapFrame`.
#[inline(always)]
unsafe fn trap_frame_get_fault_addr(regs: *const u8) -> usize {
    let r = &*(regs as *const TrapFrame);
    r.mtval
}

pub const ARCH_OPS: ArchOps = ArchOps {
    thread_ctx_size: crate::thread_ctx::thread_ctx_size,
    thread_ctx_align: crate::thread_ctx::thread_ctx_align,
    trap_frame_size: || core::mem::size_of::<TrapFrame>(),
    trap_frame_align: || core::mem::align_of::<TrapFrame>(),
    thread_ctx_init: crate::thread_ctx::thread_ctx_init,
    thread_ctx_set_sp: crate::thread_ctx::thread_ctx_set_sp,
    thread_ctx_set_tp: crate::thread_ctx::thread_ctx_set_tp,
    thread_ctx_set_ra: crate::thread_ctx::thread_ctx_set_ra,
    thread_ctx_set_retval: crate::thread_ctx::thread_ctx_set_retval,
    switch_to,
    ret_from_fork: || ret_from_fork as usize,
    trap_frame_clone,
    trap_frame_init,
    trap_frame_set_retval,
    trap_frame_set_sp,
    trap_frame_set_tp,
    current_trap_frame,
    trap_frame_get_pc,
    trap_frame_set_pc,
    trap_frame_get_nr,
    trap_frame_get_arg,
    trap_frame_get_cause,
    trap_frame_get_fault_addr,
};
