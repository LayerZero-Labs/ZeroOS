#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SyscallArgs {
    pub nr: usize,
    pub arg0: usize,
    pub arg1: usize,
    pub arg2: usize,
    pub arg3: usize,
    pub arg4: usize,
    pub arg5: usize,
}

impl SyscallArgs {
    #[inline]
    pub const fn new(
        nr: usize,
        arg0: usize,
        arg1: usize,
        arg2: usize,
        arg3: usize,
        arg4: usize,
        arg5: usize,
    ) -> Self {
        Self {
            nr,
            arg0,
            arg1,
            arg2,
            arg3,
            arg4,
            arg5,
        }
    }
}

#[derive(Clone, Copy)]
pub struct SyscallOps {
    pub handle: fn(args: &SyscallArgs, mepc: usize, frame_ptr: usize) -> isize,
}

#[cfg(feature = "syscall")]
#[inline]
pub fn handle_syscall(args: &SyscallArgs, mepc: usize, frame_ptr: usize) -> isize {
    unsafe { (crate::KERNEL.syscall.handle)(args, mepc, frame_ptr) }
}

#[cfg(feature = "syscall")]
#[inline]
pub fn syscall(
    nr: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
) -> isize {
    let args = SyscallArgs::new(nr, a0, a1, a2, a3, a4, a5);
    handle_syscall(&args, 0, 0)
}
