//! Restore user context from a saved `TrapFrame` and return with `mret`.
//!
//! This is used as a trampoline by the cooperative scheduler when starting a new thread:
//! it sets `a0 = trapframe_ptr` and jumps here.

use cfg_if::cfg_if;

#[allow(unused_imports)]
use crate::trap::TrapFrame;

cfg_if! {
    if #[cfg(target_arch = "riscv64")] {
        zeroos_macros::define_register_helpers!("ld", "ld");
    } else if #[cfg(target_arch = "riscv32")] {
        zeroos_macros::define_register_helpers!("lw", "lw");
    }
}

/// # Safety
/// `regs` must point to a valid `TrapFrame` for the thread being resumed.
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn ret_from_fork(_regs: *mut TrapFrame) -> ! {
    zeroos_macros::asm_block!(
        // Keep TrapFrame base pointer in s2 until the very end.
        "mv s2, a0",
        // Restore CSRs.
        load!(TrapFrame, t0, mepc, s2),
        load!(TrapFrame, t1, mstatus, s2),
        "csrw mepc, t0",
        "csrw mstatus, t1",
        // User TP -> mscratch (tp swap later).
        load!(TrapFrame, t6, tp, s2),
        "csrw mscratch, t6",
        // Restore GPRs (skip t0/t1/t2 and s2 for now).
        load!(TrapFrame, ra, s2),
        load!(TrapFrame, gp, s2),
        load!(TrapFrame, s0, s2),
        load!(TrapFrame, s1, s2),
        load!(TrapFrame, a0, s2),
        load!(TrapFrame, a1, s2),
        load!(TrapFrame, a2, s2),
        load!(TrapFrame, a3, s2),
        load!(TrapFrame, a4, s2),
        load!(TrapFrame, a5, s2),
        load!(TrapFrame, a6, s2),
        load!(TrapFrame, a7, s2),
        load!(TrapFrame, s3, s2),
        load!(TrapFrame, s4, s2),
        load!(TrapFrame, s5, s2),
        load!(TrapFrame, s6, s2),
        load!(TrapFrame, s7, s2),
        load!(TrapFrame, s8, s2),
        load!(TrapFrame, s9, s2),
        load!(TrapFrame, s10, s2),
        load!(TrapFrame, s11, s2),
        load!(TrapFrame, t3, s2),
        load!(TrapFrame, t4, s2),
        load!(TrapFrame, t5, s2),
        load!(TrapFrame, t6, s2),
        // Restore t0, t1, t2 late.
        load!(TrapFrame, t0, s2),
        load!(TrapFrame, t1, s2),
        load!(TrapFrame, t2, s2),
        // Switch to user stack.
        load!(TrapFrame, sp, s2),
        // Restore s2 last (we were using it as the frame base).
        load!(TrapFrame, s2, s2),
        // Swap tp/mscratch:
        // - before: tp = anchor, mscratch = user_tp
        // - after:  tp = user_tp, mscratch = anchor
        "csrrw tp, mscratch, tp",
        "mret",
    )
}
