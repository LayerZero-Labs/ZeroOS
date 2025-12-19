use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};

#[allow(unused_imports)]
use crate::ops;

pub struct Kernel {
    #[cfg(feature = "memory")]
    pub(crate) memory: ops::MemoryOps,
    #[cfg(feature = "scheduler")]
    pub(crate) scheduler: ops::SchedulerOps,
    #[cfg(feature = "trap")]
    pub(crate) trap: ops::TrapOps,
    #[cfg(feature = "vfs")]
    pub(crate) vfs: ops::VfsOps,
    #[cfg(feature = "random")]
    pub(crate) random: ops::RandomOps,
    #[cfg(feature = "arch")]
    pub(crate) arch: ops::ArchOps,
}

pub struct GlobalKernel(MaybeUninit<Kernel>);

impl GlobalKernel {
    pub const fn uninit() -> Self {
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

#[cfg(feature = "trap")]
pub fn register_trap(ops: ops::TrapOps) {
    unsafe {
        KERNEL.trap = ops;
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

#[cfg(feature = "arch")]
pub fn register_arch(ops: ops::ArchOps) {
    unsafe {
        KERNEL.arch = ops;
    }
}

/// Initialize the kernel subsystems.
pub fn init(heap_start: usize, heap_size: usize) {
    crate::kfn::memory::kinit(heap_start, heap_size);
}
