//! Platforms MUST provide `trap_handler(regs: *mut TrapFrame)` which receives

#![no_std]
#![recursion_limit = "2048"]

pub mod boot;

pub mod trap;

extern "C" {
    // Platform bootstrap hook (sets up heap, device fds, etc).
    fn __platform_bootstrap();
    // Runtime bootstrap hook (transfers into libc/runtime initialization).
    fn __runtime_bootstrap() -> !;
    // Trap entry point called by the assembly trap vector.
    pub fn trap_handler(regs: *mut TrapFrame);
}

mod riscv {
    pub use crate::boot::{__bootstrap, _start};
    pub use crate::trap::{decode_trap, PtRegs, TrapFrame, _default_trap_handler};
    pub use riscv::register::mcause::{Exception, Interrupt, Trap};
}

pub use riscv::*;
