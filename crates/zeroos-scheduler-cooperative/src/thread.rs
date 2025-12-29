use alloc::alloc::Layout;
use foundation::kfn::arch as karch;

/// Thread ID type (arch-independent).
pub type Tid = usize;

/// Arch switch context (opaque to the scheduler).
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct ThreadContext(pub *mut u8);

impl ThreadContext {
    #[inline(always)]
    pub fn as_mut_ptr(self) -> *mut u8 {
        self.0
    }
    #[inline(always)]
    pub fn as_ptr(self) -> *const u8 {
        self.0 as *const u8
    }
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
    pub thread_ctx: ThreadContext,
    pub tid: Tid,
    pub state: ThreadState,
    pub saved_pc: usize,
    pub futex_wait_addr: usize,
    pub clear_child_tid: usize,

    // Kernel stack base/size (low-level thread anchor lives at base).
    // This is conceptually independent of the scheduler; the scheduler just tracks it.
    pub kstack_base: usize,
    pub kstack_size: usize,
}

pub const KSTACK_SIZE: usize = 16 * 1024; // 16KB kernel stack

impl ThreadControlBlock {
    pub fn new(tid: Tid, user_stack_top: usize, user_tls: usize, initial_pc: usize) -> Self {
        // Allocate kernel stack (aligned) and initialize ThreadAnchor at its base
        let anchor_ptr = foundation::kfn::scheduler::kalloc_kstack(
            KSTACK_SIZE,
            karch::ktrap_frame_size(),
            karch::ktrap_frame_align(),
        );
        if anchor_ptr.is_null() {
            panic!("kalloc_kstack(KSTACK_SIZE) failed");
        }
        let anchor_addr = anchor_ptr as usize;

        // Initialize the per-thread trap frame on the kernel stack.
        let tf_addr = unsafe { foundation::kfn::scheduler::ktrap_frame_addr(anchor_ptr) };
        unsafe {
            karch::ktrap_frame_init(tf_addr as *mut u8, user_stack_top, user_tls, initial_pc);
        }

        // Allocate + init arch thread context.
        let size = karch::kthread_ctx_size();
        let align = karch::kthread_ctx_align();
        let layout = Layout::from_size_align(size, align).expect("invalid thread ctx layout");
        let ctx_ptr = foundation::kfn::memory::kzalloc(layout);
        if ctx_ptr.is_null() {
            panic!("kzalloc(thread_ctx) failed");
        }
        let kstack_top = anchor_addr + KSTACK_SIZE;
        unsafe {
            karch::kthread_ctx_init(ctx_ptr, anchor_addr, kstack_top);
        }

        Self {
            thread_ctx: ThreadContext(ctx_ptr),
            tid,
            state: ThreadState::Ready,
            saved_pc: initial_pc,
            futex_wait_addr: 0,
            clear_child_tid: 0,
            kstack_base: anchor_addr,
            kstack_size: KSTACK_SIZE,
        }
    }

    // Boot thread is initialized eagerly in `Scheduler::init()`.

    #[inline(always)]
    pub fn thread_ctx_ptr(&self) -> *const u8 {
        self.thread_ctx.as_ptr()
    }

    #[inline(always)]
    pub fn thread_ctx_ptr_mut(&mut self) -> *mut u8 {
        self.thread_ctx.as_mut_ptr()
    }
}
