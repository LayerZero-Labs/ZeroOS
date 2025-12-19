//! Platforms MUST provide `trap_handler(regs: *mut TrapFrame)` which receives

#![no_std]
#![recursion_limit = "2048"]

pub mod boot;
pub mod ops;
pub mod ret_from_fork;
pub mod switch_to;
pub mod thread_ctx;
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
    pub use crate::ops::ARCH_OPS;
    pub use crate::ret_from_fork::ret_from_fork;
    pub use crate::trap::{TrapFrame, _default_trap_handler};
    pub use foundation::kfn::thread::ThreadAnchor;
    pub use riscv::register::mcause::{Exception, Interrupt, Trap};
}

pub use riscv::*;
