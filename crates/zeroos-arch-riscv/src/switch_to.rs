//! Integer register context switch (callee-saved + core kernel regs).
//!
//! This is the RISC-V implementation of thread context switching.
//! It is intentionally kept in the arch crate (not in schedulers).

use cfg_if::cfg_if;

#[allow(unused_imports)]
use crate::thread_ctx::ThreadContext;

cfg_if! {
    if #[cfg(target_arch = "riscv64")] {
        zeroos_macros::define_register_helpers!("sd", "ld");
    } else if #[cfg(target_arch = "riscv32")] {
        zeroos_macros::define_register_helpers!("sw", "lw");
    }
}

/// # Safety
/// `old` and `new` must be valid pointers to `ThreadContext` structures.
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn switch_to(_old: *mut u8, _new: *const u8) {
    zeroos_macros::asm_block!(
        // a0 = old, a1 = new

        // Save current context
        store!(sp, { ThreadContext }(a0)),
        store!(tp, { ThreadContext }(a0)),
        store!(ra, { ThreadContext }(a0)),
        store!(gp, { ThreadContext }(a0)),
        store!(s0, { ThreadContext }(a0)),
        store!(s1, { ThreadContext }(a0)),
        store!(s2, { ThreadContext }(a0)),
        store!(s3, { ThreadContext }(a0)),
        store!(s4, { ThreadContext }(a0)),
        store!(s5, { ThreadContext }(a0)),
        store!(s6, { ThreadContext }(a0)),
        store!(s7, { ThreadContext }(a0)),
        store!(s8, { ThreadContext }(a0)),
        store!(s9, { ThreadContext }(a0)),
        store!(s10, { ThreadContext }(a0)),
        store!(s11, { ThreadContext }(a0)),
        // Restore new context
        load!(sp, { ThreadContext }(a1)),
        load!(tp, { ThreadContext }(a1)),
        load!(ra, { ThreadContext }(a1)),
        load!(gp, { ThreadContext }(a1)),
        load!(s0, { ThreadContext }(a1)),
        load!(s1, { ThreadContext }(a1)),
        load!(s2, { ThreadContext }(a1)),
        load!(s3, { ThreadContext }(a1)),
        load!(s4, { ThreadContext }(a1)),
        load!(s5, { ThreadContext }(a1)),
        load!(s6, { ThreadContext }(a1)),
        load!(s7, { ThreadContext }(a1)),
        load!(s8, { ThreadContext }(a1)),
        load!(s9, { ThreadContext }(a1)),
        load!(s10, { ThreadContext }(a1)),
        load!(s11, { ThreadContext }(a1)),
        // Return value for the yielding thread
        load!(a0, { ThreadContext.retval }(a1)),
        "ret",
    )
}
