//! Platforms MUST provide `trap_handler(regs: *mut TrapFrame)` - this crate only provides the entry/exit wrapper.

use cfg_if::cfg_if;

pub use riscv::register::mcause::{Exception, Interrupt, Trap};

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct TrapFrame {
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
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
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,

    pub mepc: usize,
    pub mstatus: usize,
    pub mcause: usize,
    pub mtval: usize,
}

pub type PtRegs = TrapFrame;

#[allow(non_camel_case_types)]
pub type pt_regs = TrapFrame;

impl foundation::ArchContext for TrapFrame {
    fn new() -> Self {
        Self {
            ra: 0,
            sp: 0,
            gp: 0,
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
            mstatus: 0,
            mcause: 0,
            mtval: 0,
        }
    }
    fn sp(&self) -> usize {
        self.sp
    }
    fn set_sp(&mut self, sp: usize) {
        self.sp = sp;
    }
    fn tp(&self) -> usize {
        self.tp
    }
    fn set_tp(&mut self, tp: usize) {
        self.tp = tp;
    }
    fn return_value(&self) -> usize {
        self.a0
    }
    fn set_return_value(&mut self, val: usize) {
        self.a0 = val;
    }
    fn ra(&self) -> usize {
        self.ra
    }
    fn set_ra(&mut self, ra: usize) {
        self.ra = ra;
    }
    fn gp(&self) -> usize {
        self.gp
    }
    fn set_gp(&mut self, gp: usize) {
        self.gp = gp;
    }
    unsafe fn read_from_ptr(ptr: *const Self) -> Self {
        *ptr
    }
    unsafe fn write_to_ptr(&self, ptr: *mut Self) {
        *ptr = *self;
    }
}

impl foundation::FramePointerContext for TrapFrame {
    #[inline(always)]
    fn set_frame_pointer(&mut self, fp: usize) {
        self.s0 = fp;
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
        zeroos_macros::define_register_helpers!(crate::TrapFrame, "sd", "ld");
    } else if #[cfg(target_arch = "riscv32")] {
        zeroos_macros::define_register_helpers!(crate::TrapFrame, "sw", "lw");
    }
}

use core::arch::global_asm;

mod imp {
    use super::*;
    use zeroos_macros::asm_block;

    #[unsafe(naked)]
    #[no_mangle]
    /// # Safety
    /// Trap entrypoint; must only be called from the trap path with a valid stack/context.
    pub unsafe extern "C" fn save_regs() -> *mut PtRegs {
        asm_block!(
            "addi sp, sp, -{PTREGS_SIZE}",
            store!(t0),
            "addi t0, sp, {PTREGS_SIZE}",

            "csrr t1, mscratch",
            store!(t1, ra),
            store!(t0, sp),
            store!(gp),
            store!(tp),
            store!(t1),
            store!(t2),
            store!(s0),
            store!(s1),
            store!(a0),
            store!(a1),
            store!(a2),
            store!(a3),
            store!(a4),
            store!(a5),
            store!(a6),
            store!(a7),
            store!(s2),
            store!(s3),
            store!(s4),
            store!(s5),
            store!(s6),
            store!(s7),
            store!(s8),
            store!(s9),
            store!(s10),
            store!(s11),
            store!(t3),
            store!(t4),
            store!(t5),
            store!(t6),
            "csrr t0, mepc",
            "csrr t1, mstatus",
            "csrr t2, mcause",
            "csrr t3, mtval",
            store!(t0, mepc),
            store!(t1, mstatus),
            store!(t2, mcause),
            store!(t3, mtval),
            "mv a0, sp",
            "ret",
            PTREGS_SIZE = const core::mem::size_of::<PtRegs>(),
        );
    }

    #[unsafe(naked)]
    #[no_mangle]
    /// # Safety
    /// `regs` must point to a valid `PtRegs` for the current trap context.
    pub unsafe extern "C" fn restore_regs(regs: *mut PtRegs) -> ! {
        asm_block!(
            "mv s0, a0",
            load!(t0, mepc, s0),
            load!(t1, mstatus, s0),
            "csrw mepc, t0",
            "csrw mstatus, t1",
            load!(ra, ra, s0),
            load!(gp, gp, s0),
            load!(tp, tp, s0),
            load!(t0, t0, s0),
            load!(t1, t1, s0),
            load!(t2, t2, s0),
            load!(s1, s1, s0),
            load!(a0, a0, s0),
            load!(a1, a1, s0),
            load!(a2, a2, s0),
            load!(a3, a3, s0),
            load!(a4, a4, s0),
            load!(a5, a5, s0),
            load!(a6, a6, s0),
            load!(a7, a7, s0),
            load!(s2, s2, s0),
            load!(s3, s3, s0),
            load!(s4, s4, s0),
            load!(s5, s5, s0),
            load!(s6, s6, s0),
            load!(s7, s7, s0),
            load!(s8, s8, s0),
            load!(s9, s9, s0),
            load!(s10, s10, s0),
            load!(s11, s11, s0),
            load!(t3, t3, s0),
            load!(t4, t4, s0),
            load!(t5, t5, s0),
            load!(t6, t6, s0),
            load!(sp, sp, s0),
            load!(s0, s0, s0),
            "mret",
        );
    }

    #[unsafe(naked)]
    #[no_mangle]
    /// # Safety
    /// Trap entrypoint; must only be invoked by the CPU trap vector with a valid trap context.
    pub unsafe extern "C" fn _default_trap_handler() {
        asm_block!(

            "csrw mscratch, ra",
            "call {save_regs}",
            "mv s0, a0",
            "call {trap_handler}",
            "mv a0, s0",
            "tail {restore_regs}",
            save_regs = sym save_regs,
            restore_regs = sym restore_regs,
            trap_handler = sym crate::trap_handler,
        );
    }
}

pub use imp::{_default_trap_handler, restore_regs, save_regs};

global_asm!(
    ".align 2",
    ".weak _trap_handler",
    ".type  _trap_handler, @function",
    "_trap_handler:",
    "j {default}",
    default = sym imp::_default_trap_handler,
);

#[inline]
pub fn decode_trap(mcause: usize) -> Trap {
    let is_int = (mcause & (1 << (usize::BITS - 1))) != 0;
    let code = mcause & !(1 << (usize::BITS - 1));
    if is_int {
        match code {
            1 => Trap::Interrupt(Interrupt::SupervisorSoft),
            3 => Trap::Interrupt(Interrupt::MachineSoft),
            5 => Trap::Interrupt(Interrupt::SupervisorTimer),
            7 => Trap::Interrupt(Interrupt::MachineTimer),
            9 => Trap::Interrupt(Interrupt::SupervisorExternal),
            11 => Trap::Interrupt(Interrupt::MachineExternal),
            _ => Trap::Interrupt(Interrupt::Unknown),
        }
    } else {
        match code {
            0 => Trap::Exception(Exception::InstructionMisaligned),
            1 => Trap::Exception(Exception::InstructionFault),
            2 => Trap::Exception(Exception::IllegalInstruction),
            3 => Trap::Exception(Exception::Breakpoint),
            4 => Trap::Exception(Exception::LoadMisaligned),
            5 => Trap::Exception(Exception::LoadFault),
            6 => Trap::Exception(Exception::StoreMisaligned),
            7 => Trap::Exception(Exception::StoreFault),
            8 => Trap::Exception(Exception::UserEnvCall),
            9 => Trap::Exception(Exception::SupervisorEnvCall),
            11 => Trap::Exception(Exception::MachineEnvCall),
            12 => Trap::Exception(Exception::InstructionPageFault),
            13 => Trap::Exception(Exception::LoadPageFault),
            15 => Trap::Exception(Exception::StorePageFault),
            _ => Trap::Exception(Exception::Unknown),
        }
    }
}
