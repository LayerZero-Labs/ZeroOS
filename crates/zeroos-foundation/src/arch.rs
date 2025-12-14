/// by the scheduler and syscall handlers. The actual struct layout is
pub trait ArchContext: Clone + Copy + Sized {
    fn new() -> Self;

    fn sp(&self) -> usize;
    fn set_sp(&mut self, sp: usize);

    fn tp(&self) -> usize;
    fn set_tp(&mut self, tp: usize);

    fn return_value(&self) -> usize;
    fn set_return_value(&mut self, val: usize);

    fn ra(&self) -> usize;
    fn set_ra(&mut self, ra: usize);

    fn gp(&self) -> usize {
        0
    }
    fn set_gp(&mut self, _gp: usize) {}

    /// # Safety
    /// Pointer must be valid and properly aligned.
    unsafe fn read_from_ptr(ptr: *const Self) -> Self;

    /// # Safety
    /// Pointer must be valid and properly aligned.
    unsafe fn write_to_ptr(&self, ptr: *mut Self);
}

pub trait SyscallFrame: Sized {
    fn pc(&self) -> usize;
    fn syscall_number(&self) -> usize;
    fn arg(&self, idx: usize) -> usize;
    fn set_ret(&mut self, ret: isize);
}

pub trait FramePointerContext {
    fn set_frame_pointer(&mut self, fp: usize);
}
