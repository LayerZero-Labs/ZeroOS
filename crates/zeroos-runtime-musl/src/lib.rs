#![no_std]

#[cfg(feature = "backtrace")]
mod eh_frame_register;
mod lock_override;
mod stack;

pub use stack::build_musl_stack;

#[cfg(target_arch = "riscv64")]
pub mod riscv64;
