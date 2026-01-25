//! Platforms must provide `trap_handler(regs: *mut TrapFrame)`; this crate provides the entry/exit wrapper.
#[cfg(not(target_os = "none"))]
use core::arch::global_asm;

use cfg_if::cfg_if;

pub use riscv::register::mcause::{Exception, Interrupt, Trap};

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct TrapFrame {
    // Integer registers (x0 is hardwired to zero and not stored).
    pub ra: usize,  // x1
    pub sp: usize,  // x2
    pub gp: usize,  // x3
    pub tp: usize,  // x4
    pub t0: usize,  // x5
    pub t1: usize,  // x6
    pub t2: usize,  // x7
    pub s0: usize,  // x8
    pub s1: usize,  // x9
    pub a0: usize,  // x10
    pub a1: usize,  // x11
    pub a2: usize,  // x12
    pub a3: usize,  // x13
    pub a4: usize,  // x14
    pub a5: usize,  // x15
    pub a6: usize,  // x16
    pub a7: usize,  // x17
    pub s2: usize,  // x18
    pub s3: usize,  // x19
    pub s4: usize,  // x20
    pub s5: usize,  // x21
    pub s6: usize,  // x22
    pub s7: usize,  // x23
    pub s8: usize,  // x24
    pub s9: usize,  // x25
    pub s10: usize, // x26
    pub s11: usize, // x27
    pub t3: usize,  // x28
    pub t4: usize,  // x29
    pub t5: usize,  // x30
    pub t6: usize,  // x31

    pub mepc: usize,
    pub mstatus: usize,
    pub mcause: usize,
    pub mtval: usize,

    pub from_kernel: usize,
}

#[allow(non_camel_case_types)]
pub type TrapFramePtr = *mut TrapFrame;

impl Default for TrapFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl TrapFrame {
    pub fn new() -> Self {
        let mut mstatus: usize = 0;
        // Set MPP = 3 (Machine mode) so mret returns to M-mode by default.
        mstatus |= 3 << 11;

        let current_gp: usize;
        unsafe {
            core::arch::asm!("mv {}, gp", out(reg) current_gp);
        }

        Self {
            ra: 0,
            sp: 0,
            gp: current_gp,
            tp: 0,
            t0: 0,
            t1: 0,
            t2: 0,
            s0: 0,
            s1: 0,
            a0: 0,
            a1: 0,
            a2: 0,
            a3: 0,
            a4: 0,
            a5: 0,
            a6: 0,
            a7: 0,
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
            t3: 0,
            t4: 0,
            t5: 0,
            t6: 0,
            mepc: 0,
            mstatus,
            mcause: 0,
            mtval: 0,
            from_kernel: 0,
        }
    }

    /// # Safety
    /// `ptr` must point to a valid, aligned region of at least `size_of::<TrapFrame>()` bytes.
    pub unsafe fn write_to_ptr(&self, ptr: *mut Self) {
        *ptr = *self;
    }
}

impl foundation::SyscallFrame for TrapFrame {
    #[inline(always)]
    fn pc(&self) -> usize {
        self.mepc
    }

    #[inline(always)]
    fn syscall_number(&self) -> usize {
        self.a7
    }

    #[inline(always)]
    fn arg(&self, idx: usize) -> usize {
        match idx {
            0 => self.a0,
            1 => self.a1,
            2 => self.a2,
            3 => self.a3,
            4 => self.a4,
            5 => self.a5,
            _ => 0,
        }
    }

    #[inline(always)]
    fn set_ret(&mut self, ret: isize) {
        self.a0 = ret as usize;
    }
}

cfg_if! {
    if #[cfg(target_arch = "riscv64")] {
        zeroos_macros::define_register_helpers!("sd", "ld");
    } else if #[cfg(target_arch = "riscv32")] {
        zeroos_macros::define_register_helpers!("sw", "lw");
    }
}

mod imp {
    use super::*;

    /// # Safety
    /// This is the low-level trap entry point. It must be called by the CPU
    /// hardware vector with a valid `tp` (if from user) or valid kernel stack.
    #[unsafe(naked)]
    #[no_mangle]
    pub unsafe extern "C" fn _default_trap_handler() -> ! {
        #[allow(unused_imports)]
        use crate::trap::TrapFrame;
        #[allow(unused_imports)]
        use foundation::kfn::thread::ThreadAnchor;

        cfg_if! {
            if #[cfg(target_arch = "riscv64")] {
                zeroos_macros::asm_block!(
                    "csrrw tp, mscratch, tp",
                    "bnez tp, .Lsave_context",

                        ".Lrestore_kernel_tpsp:",
                        "csrr tp, mscratch",
                        store!(t6, {ThreadAnchor.stash0}(tp) @k),
                        "li t6, 1",
                        store!(sp, {ThreadAnchor.kernel_sp}(tp)),
                        "j .Lcommon_save_context",

                        ".Lsave_context:",
                        store!(t6, {ThreadAnchor.stash0}(tp) @u),
                        "li t6, 0",

                        ".Lcommon_save_context:",
                        store!(sp, {ThreadAnchor.user_sp}(tp)),
                        load!(sp, {ThreadAnchor.kernel_sp}(tp)),
                        "addi sp, sp, -{FRAME_SIZE}",

                    store!(ra, {TrapFrame}(sp)),
                    store!(gp, {TrapFrame}(sp)),
                    store!(t0, {TrapFrame}(sp)),
                    store!(t1, {TrapFrame}(sp)),
                    store!(t2, {TrapFrame}(sp)),
                    store!(s0, {TrapFrame}(sp)),
                    store!(s1, {TrapFrame}(sp)),
                    store!(a0, {TrapFrame}(sp)),
                    store!(a1, {TrapFrame}(sp)),
                    store!(a2, {TrapFrame}(sp)),
                    store!(a3, {TrapFrame}(sp)),
                    store!(a4, {TrapFrame}(sp)),
                    store!(a5, {TrapFrame}(sp)),
                    store!(a6, {TrapFrame}(sp)),
                    store!(a7, {TrapFrame}(sp)),
                    store!(s2, {TrapFrame}(sp)),
                    store!(s3, {TrapFrame}(sp)),
                    store!(s4, {TrapFrame}(sp)),
                    store!(s5, {TrapFrame}(sp)),
                    store!(s6, {TrapFrame}(sp)),
                    store!(s7, {TrapFrame}(sp)),
                    store!(s8, {TrapFrame}(sp)),
                    store!(s9, {TrapFrame}(sp)),
                    store!(s10, {TrapFrame}(sp)),
                    store!(s11, {TrapFrame}(sp)),
                    store!(t3, {TrapFrame}(sp)),
                    store!(t4, {TrapFrame}(sp)),
                    store!(t5, {TrapFrame}(sp)),

                        store!(t6, {TrapFrame.from_kernel}(sp)),

                        load!(t6, {ThreadAnchor.stash0}(tp) @restore),
                        store!(t6, {TrapFrame}(sp)),

                        load!(s0, {ThreadAnchor.user_sp}(tp)),
                        "csrr s1, mstatus",
                        "csrr s2, mepc",
                        "csrr s3, mtval",
                        "csrr s4, mcause",
                        "csrr s5, mscratch",
                        store!(s0, {TrapFrame.sp}(sp)),
                        store!(s1, {TrapFrame.mstatus}(sp)),
                        store!(s2, {TrapFrame.mepc}(sp)),
                        store!(s3, {TrapFrame.mtval}(sp)),
                        store!(s4, {TrapFrame.mcause}(sp)),
                        store!(s5, {TrapFrame.tp}(sp)),

                        "csrw mscratch, x0",

                        "mv a0, sp",
                        "call {trap_handler}",
                        "j ret_from_exception",

                        "ret_from_exception:",
                        load!(t6, {TrapFrame.from_kernel}(sp)),
                        "bnez t6, 1f",

                        "addi s0, sp, {FRAME_SIZE}",
                        store!(s0, {ThreadAnchor.kernel_sp}(tp)),
                        "csrw mscratch, tp",

                    "1:",
                    load!(a0, {TrapFrame.mstatus}(sp)),
                    load!(a2, {TrapFrame.mepc}(sp)),
                    "csrw mstatus, a0",
                    "csrw mepc, a2",

                    load!(ra, {TrapFrame}(sp)),
                    load!(gp, {TrapFrame}(sp)),
                    load!(tp, {TrapFrame}(sp)),
                    load!(t0, {TrapFrame}(sp)),

                    load!(t1, {TrapFrame}(sp)),
                    load!(t2, {TrapFrame}(sp)),
                    load!(s0, {TrapFrame}(sp)),
                    load!(s1, {TrapFrame}(sp)),
                    load!(a0, {TrapFrame}(sp)),
                    load!(a1, {TrapFrame}(sp)),
                    load!(a2, {TrapFrame}(sp)),
                    load!(a3, {TrapFrame}(sp)),
                    load!(a4, {TrapFrame}(sp)),
                    load!(a5, {TrapFrame}(sp)),
                    load!(a6, {TrapFrame}(sp)),
                    load!(a7, {TrapFrame}(sp)),
                    load!(s2, {TrapFrame}(sp)),
                    load!(s3, {TrapFrame}(sp)),
                    load!(s4, {TrapFrame}(sp)),
                    load!(s5, {TrapFrame}(sp)),
                    load!(s6, {TrapFrame}(sp)),
                    load!(s7, {TrapFrame}(sp)),
                    load!(s8, {TrapFrame}(sp)),
                    load!(s9, {TrapFrame}(sp)),
                    load!(s10, {TrapFrame}(sp)),
                    load!(s11, {TrapFrame}(sp)),
                    load!(t3, {TrapFrame}(sp)),
                    load!(t4, {TrapFrame}(sp)),
                    load!(t5, {TrapFrame}(sp)),
                    load!(t6, {TrapFrame}(sp)),

                    load!(sp, {TrapFrame}(sp)),
                    "mret",

                    FRAME_SIZE = const core::mem::size_of::<TrapFrame>(),
                    trap_handler = sym crate::trap_handler,
                );
            } else {
                 core::arch::naked_asm!("unimp");
            }
        }
    }
}

pub use imp::_default_trap_handler;

#[cfg(not(target_os = "none"))]
global_asm!(
    ".align 2",
    ".weak _trap_handler",
    ".type  _trap_handler, @function",
    "_trap_handler:",
    "j {default}",
    default = sym imp::_default_trap_handler,
);
