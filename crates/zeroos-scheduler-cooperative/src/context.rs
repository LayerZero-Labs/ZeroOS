zeroos_macros::require_exactly_one_feature!("riscv");

#[cfg(feature = "riscv")]
pub type TrapFrame = arch_riscv::TrapFrame;

pub type Context = TrapFrame;
