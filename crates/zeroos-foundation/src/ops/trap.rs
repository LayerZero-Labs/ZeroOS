#[derive(Clone, Copy)]
pub struct TrapOps {
    pub handle_syscall: fn(
        nr: usize,
        a0: usize,
        a1: usize,
        a2: usize,
        a3: usize,
        a4: usize,
        a5: usize,
        mepc: usize,
        frame_ptr: usize,
    ) -> isize,

    pub handle_exception: fn(code: usize, mepc: usize, mtval: usize) -> Option<usize>,
    pub handle_interrupt: fn(code: usize),
}

impl Default for TrapOps {
    fn default() -> Self {
        Self {
            handle_syscall: |_, _, _, _, _, _, _, _, _| -38,
            handle_exception: |_, _, _| None,
            handle_interrupt: |_| {},
        }
    }
}
