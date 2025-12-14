use super::context::Context;
use foundation::ArchContext;

pub type Tid = usize;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ThreadContext {
    sp: usize,
    tp: usize,
    ra: usize,
    gp: usize,
    retval: usize,
}

impl ArchContext for ThreadContext {
    fn new() -> Self {
        Self {
            sp: 0,
            tp: 0,
            ra: 0,
            gp: 0,
            retval: 0,
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
        self.retval
    }

    fn set_return_value(&mut self, val: usize) {
        self.retval = val;
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

#[inline(always)]
pub fn sync_thread_ctx_from_frame<C: ArchContext>(thread_ctx: &mut ThreadContext, frame: &C) {
    thread_ctx.set_sp(frame.sp());
    thread_ctx.set_tp(frame.tp());
    thread_ctx.set_ra(frame.ra());
    thread_ctx.set_gp(frame.gp());
    thread_ctx.set_return_value(frame.return_value());
}

#[inline(always)]
pub fn apply_thread_ctx_to_frame<C: ArchContext>(frame: &mut C, thread_ctx: &ThreadContext) {
    frame.set_sp(thread_ctx.sp());
    frame.set_tp(thread_ctx.tp());
    frame.set_ra(thread_ctx.ra());
    frame.set_gp(thread_ctx.gp());
    frame.set_return_value(thread_ctx.return_value());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Running,
    Blocked,
    Exited,
}

#[repr(C)]
pub struct ThreadControlBlock {
    pub trap_frame: Context,
    pub thread_ctx: ThreadContext,

    pub tid: Tid,

    pub state: ThreadState,

    pub saved_pc: usize,

    pub futex_wait_addr: usize,

    pub clear_child_tid: usize,
}

impl ThreadControlBlock {
    pub fn new(tid: Tid, initial_sp: usize, tls: usize, initial_pc: usize) -> Self {
        let mut trap_frame: Context = Context::new();
        let mut thread_ctx: ThreadContext = ThreadContext::new();
        thread_ctx.set_sp(initial_sp);
        thread_ctx.set_tp(tls);
        trap_frame.set_sp(thread_ctx.sp());
        trap_frame.set_tp(thread_ctx.tp());
        trap_frame.set_ra(thread_ctx.ra());
        trap_frame.set_gp(thread_ctx.gp());
        trap_frame.set_return_value(thread_ctx.return_value());

        Self {
            trap_frame,
            thread_ctx,
            tid,
            state: ThreadState::Ready,
            saved_pc: initial_pc,
            futex_wait_addr: 0,
            clear_child_tid: 0,
        }
    }
}
