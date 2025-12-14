#![no_std]

extern crate alloc;

use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};

pub mod arch;
pub mod entry;
pub mod kfn;
pub mod ops;
pub mod syscall;
pub mod utils;

pub use arch::{ArchContext, FramePointerContext, SyscallFrame};

pub use entry::__main_entry;

pub struct Kernel {
    #[cfg(feature = "memory")]
    pub memory: ops::MemoryOps,
    #[cfg(feature = "scheduler")]
    pub scheduler: ops::SchedulerOps,
    #[cfg(feature = "syscall")]
    pub syscall: syscall::SyscallOps,
    #[cfg(feature = "vfs")]
    pub vfs: ops::VfsOps,
    #[cfg(feature = "random")]
    pub random: ops::RandomOps,
}

pub struct GlobalKernel(MaybeUninit<Kernel>);

impl GlobalKernel {
    const fn uninit() -> Self {
        Self(MaybeUninit::uninit())
    }
}

impl Deref for GlobalKernel {
    type Target = Kernel;
    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.assume_init_ref() }
    }
}

impl DerefMut for GlobalKernel {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.assume_init_mut() }
    }
}

pub static mut KERNEL: GlobalKernel = GlobalKernel::uninit();

#[cfg(feature = "memory")]
pub fn register_memory(ops: ops::MemoryOps) {
    unsafe {
        KERNEL.memory = ops;
    }
}

#[cfg(feature = "scheduler")]
pub fn register_scheduler(ops: ops::SchedulerOps) {
    unsafe {
        KERNEL.scheduler = ops;
    }
}

#[cfg(feature = "syscall")]
pub fn register_syscall(ops: syscall::SyscallOps) {
    unsafe {
        KERNEL.syscall = ops;
    }
}

#[cfg(feature = "vfs")]
pub fn register_vfs(ops: ops::VfsOps) {
    unsafe {
        KERNEL.vfs = ops;
    }
}

#[cfg(feature = "random")]
pub fn register_random(ops: ops::RandomOps) {
    unsafe {
        KERNEL.random = ops;
    }
}

#[cfg(feature = "memory")]
pub fn init(heap_start: usize, heap_size: usize) {
    unsafe {
        (KERNEL.memory.init)(heap_start, heap_size);
    }
}
