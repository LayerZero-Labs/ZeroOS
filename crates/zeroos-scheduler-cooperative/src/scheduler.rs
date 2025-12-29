use crate::thread::{ThreadControlBlock, ThreadState, Tid};
use alloc::boxed::Box;
use core::ptr::NonNull;
use foundation::utils::GlobalOption;

use libc::{EAGAIN, EDEADLK, EPERM};

use alloc::alloc::Layout;
use foundation::kfn::arch as karch;

pub const MAX_THREADS: usize = 64;

static SCHEDULER: GlobalOption<Scheduler> = GlobalOption::none();

pub struct Scheduler {
    pub(crate) threads: [Option<NonNull<ThreadControlBlock>>; MAX_THREADS],
    pub(crate) thread_count: usize,
    pub(crate) current_index: usize,
    pub(crate) next_tid: Tid,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            threads: [None; MAX_THREADS],
            thread_count: 0,
            current_index: 0,
            next_tid: 1,
        }
    }

    pub fn init() -> usize {
        let anchor_ptr = foundation::kfn::scheduler::kalloc_kstack(
            crate::thread::KSTACK_SIZE,
            karch::ktrap_frame_size(),
            karch::ktrap_frame_align(),
        );
        if anchor_ptr.is_null() {
            panic!("kalloc_kstack(KSTACK_SIZE) failed for boot thread");
        }

        SCHEDULER.set(Scheduler::new());

        Scheduler::with_mut(|scheduler| {
            // Create the boot TCB (tid=1) eagerly.
            // Note: we use kmalloc + ptr::write instead of Box::new to avoid triggering
            // syscalls (via musl malloc) before the runtime is fully initialized.
            let boot_layout = core::alloc::Layout::new::<ThreadControlBlock>();
            let boot_ptr = foundation::kfn::memory::kmalloc(boot_layout) as *mut ThreadControlBlock;
            if boot_ptr.is_null() {
                panic!("kmalloc(ThreadControlBlock) failed for boot thread");
            }

            unsafe {
                core::ptr::write(
                    boot_ptr,
                    ThreadControlBlock {
                        thread_ctx: crate::thread::ThreadContext(core::ptr::null_mut()),
                        tid: 1,
                        state: ThreadState::Running,
                        saved_pc: 0,
                        futex_wait_addr: 0,
                        clear_child_tid: 0,
                        kstack_base: anchor_ptr as usize,
                        kstack_size: crate::thread::KSTACK_SIZE,
                    },
                );
            }

            let mut boot = unsafe { Box::from_raw(boot_ptr) };

            // Kernel context uses tp=anchor and sp=top-of-kstack.
            {
                // Initialize the boot trap frame on the kernel stack.
                let tf_addr = unsafe { foundation::kfn::scheduler::ktrap_frame_addr(anchor_ptr) };
                unsafe {
                    karch::ktrap_frame_init(tf_addr as *mut u8, 0, 0, 0);
                }

                // Allocate + init arch thread context for boot.
                let size = karch::kthread_ctx_size();
                let align = karch::kthread_ctx_align();
                let layout =
                    Layout::from_size_align(size, align).expect("invalid thread ctx layout");
                let ctx_ptr = foundation::kfn::memory::kzalloc(layout);
                if ctx_ptr.is_null() {
                    panic!("kzalloc(thread_ctx) failed for boot");
                }
                let anchor_addr = anchor_ptr as usize;
                let kstack_top = anchor_addr + crate::thread::KSTACK_SIZE;
                unsafe {
                    karch::kthread_ctx_init(ctx_ptr, anchor_addr, kstack_top);
                }
                boot.thread_ctx = crate::thread::ThreadContext(ctx_ptr);
            }

            let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(boot)) };
            scheduler.threads[0] = Some(ptr);
            scheduler.thread_count = 1;
            scheduler.current_index = 0;
            scheduler.next_tid = 2;

            unsafe {
                (*anchor_ptr).task_ptr = ptr.as_ptr() as usize;
            }
        });

        anchor_ptr as usize
    }

    #[inline(always)]
    pub fn with_mut<R>(f: impl FnOnce(&mut Scheduler) -> R) -> Option<R> {
        SCHEDULER.with_some_mut(f)
    }

    pub fn current_thread(&self) -> Option<NonNull<ThreadControlBlock>> {
        if self.current_index < self.thread_count {
            self.threads[self.current_index]
        } else {
            None
        }
    }

    pub fn thread_count(&self) -> usize {
        self.thread_count
    }

    pub fn current_tid_or_1(&self) -> usize {
        if let Some(tcb) = self.current_thread() {
            unsafe { (*tcb.as_ptr()).tid }
        } else {
            1
        }
    }

    pub fn yield_now(&mut self) {
        if let Some(tcb) = self.current_thread() {
            unsafe {
                karch::kthread_ctx_set_retval((*tcb.as_ptr()).thread_ctx_ptr_mut(), 0);
            }
        }
        if self.thread_count == 0 {
            return;
        }

        let current_idx = self.current_index;

        if let Some(current_tcb) = self.threads[current_idx] {
            unsafe {
                if (*current_tcb.as_ptr()).state == ThreadState::Running {
                    (*current_tcb.as_ptr()).state = ThreadState::Ready;
                }
            }
        }

        let Some(next_idx) = self.find_next_ready((current_idx + 1) % self.thread_count) else {
            if let Some(current_tcb) = self.threads[current_idx] {
                unsafe {
                    if (*current_tcb.as_ptr()).state == ThreadState::Ready {
                        (*current_tcb.as_ptr()).state = ThreadState::Running;
                    }
                }
            }
            return;
        };

        if next_idx == current_idx {
            if let Some(current_tcb) = self.threads[current_idx] {
                unsafe {
                    (*current_tcb.as_ptr()).state = ThreadState::Running;
                }
            }
            return;
        }

        if let Some(next_tcb) = self.threads[next_idx] {
            unsafe {
                (*next_tcb.as_ptr()).state = ThreadState::Running;
            }
            self.current_index = next_idx;
        }

        // Perform context switch if needed
        unsafe {
            if let (Some(mut old_ptr), Some(new_ptr)) =
                (self.threads[current_idx], self.threads[self.current_index])
            {
                let old_tcb = old_ptr.as_mut();
                let new_tcb = new_ptr.as_ref();
                karch::kswitch_to(old_tcb.thread_ctx_ptr_mut(), new_tcb.thread_ctx_ptr());
            }
        }
    }

    pub fn wait_on_addr(&mut self, addr: usize, expected: i32) -> isize {
        let actual = unsafe { core::ptr::read_volatile(addr as *const i32) };
        if actual != expected {
            if let Some(tcb) = self.current_thread() {
                unsafe {
                    karch::kthread_ctx_set_retval(
                        (*tcb.as_ptr()).thread_ctx_ptr_mut(),
                        (-EAGAIN as isize) as usize,
                    );
                }
            }
            return -EAGAIN as isize;
        }

        if self.thread_count() <= 1 {
            if let Some(tcb) = self.current_thread() {
                unsafe {
                    karch::kthread_ctx_set_retval(
                        (*tcb.as_ptr()).thread_ctx_ptr_mut(),
                        (-EDEADLK as isize) as usize,
                    );
                }
            }
            return -EDEADLK as isize;
        }

        if let Some(tcb) = self.current_thread() {
            unsafe {
                karch::kthread_ctx_set_retval((*tcb.as_ptr()).thread_ctx_ptr_mut(), 0);
            }
        }

        if let Some(current_tcb) = self.current_thread() {
            unsafe {
                (*current_tcb.as_ptr()).state = ThreadState::Blocked;
                (*current_tcb.as_ptr()).futex_wait_addr = addr;
            }
            self.yield_now();
        }
        0
    }

    pub fn wake_on_addr(&mut self, addr: usize, count: usize) -> usize {
        let ret = self.wake_futex(addr, count);
        if let Some(tcb) = self.current_thread() {
            unsafe {
                karch::kthread_ctx_set_retval((*tcb.as_ptr()).thread_ctx_ptr_mut(), ret);
            }
        }
        ret
    }

    pub fn spawn_thread(
        &mut self,
        parent_frame_ptr: usize,
        stack: usize,
        tls: usize,
        clear_child_tid_ptr: usize,
        mepc: usize,
    ) -> isize {
        if self.thread_count == 0 {
            // Scheduler must be initialized (boot TCB installed) before spawning threads.
            return -EPERM as isize;
        }

        let new_tid = self.next_tid;
        self.next_tid += 1;
        let stack_base = stack & !0xF;

        let mut child_tcb = Box::new(crate::thread::ThreadControlBlock::new(
            new_tid, stack_base, tls, mepc,
        ));

        // Prepare child state
        {
            let anchor_ptr =
                child_tcb.kstack_base as *const foundation::kfn::scheduler::ThreadAnchor;
            let tf_addr = unsafe { foundation::kfn::scheduler::ktrap_frame_addr(anchor_ptr) };

            // Clone parent trap frame into child (opaque to scheduler).
            unsafe {
                karch::ktrap_frame_clone(tf_addr as *mut u8, parent_frame_ptr as *const u8);

                // Update child's stack pointer in the trap frame if a new stack was provided.
                if stack != 0 {
                    karch::ktrap_frame_set_sp(tf_addr as *mut u8, stack_base);
                }
                // Update child's TLS pointer in the trap frame if provided.
                if tls != 0 {
                    karch::ktrap_frame_set_tp(tf_addr as *mut u8, tls);
                }

                // Child return value = 0.
                karch::ktrap_frame_set_retval(tf_addr as *mut u8, 0);

                // Link anchor -> TCB.
                let anchor_ptr =
                    child_tcb.kstack_base as *mut foundation::kfn::scheduler::ThreadAnchor;
                (*anchor_ptr).task_ptr = Box::as_ref(&child_tcb) as *const _ as usize;

                // Bootstrap the child by returning into arch `ret_from_fork(tf_ptr)`.
                karch::kthread_ctx_set_retval(child_tcb.thread_ctx_ptr_mut(), tf_addr);
                karch::kthread_ctx_set_ra(child_tcb.thread_ctx_ptr_mut(), karch::kret_from_fork());
            }
        }

        child_tcb.clear_child_tid = clear_child_tid_ptr;

        let child_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(child_tcb)) };
        if self.thread_count >= MAX_THREADS {
            return -EPERM as isize;
        }
        self.threads[self.thread_count] = Some(child_ptr);
        self.thread_count += 1;

        if let Some(parent_tcb) = self.current_thread() {
            unsafe {
                karch::kthread_ctx_set_retval((*parent_tcb.as_ptr()).thread_ctx_ptr_mut(), new_tid);
            }
        }

        new_tid as isize
    }

    fn find_next_ready(&self, start_from: usize) -> Option<usize> {
        for i in start_from..self.thread_count {
            if let Some(tcb) = self.threads[i] {
                if unsafe { (*tcb.as_ptr()).state == ThreadState::Ready } {
                    return Some(i);
                }
            }
        }
        for i in 0..start_from {
            if let Some(tcb) = self.threads[i] {
                if unsafe { (*tcb.as_ptr()).state == ThreadState::Ready } {
                    return Some(i);
                }
            }
        }
        None
    }

    pub fn wake_futex(&mut self, futex_addr: usize, max_count: usize) -> usize {
        let mut woken = 0;

        for i in 0..self.thread_count {
            if woken >= max_count {
                break;
            }
            if let Some(tcb) = self.threads[i] {
                unsafe {
                    if (*tcb.as_ptr()).state == ThreadState::Blocked
                        && (*tcb.as_ptr()).futex_wait_addr == futex_addr
                    {
                        (*tcb.as_ptr()).state = ThreadState::Ready;
                        (*tcb.as_ptr()).futex_wait_addr = 0;
                        woken += 1;
                    }
                }
            }
        }

        woken
    }

    pub fn exit_current_and_yield(&mut self, exit_code: i32) -> isize {
        if let Some(current_tcb) = self.current_thread() {
            let is_main_thread = unsafe { (*current_tcb.as_ptr()).tid == 1 };

            unsafe {
                (*current_tcb.as_ptr()).state = ThreadState::Exited;

                let clear = (*current_tcb.as_ptr()).clear_child_tid;
                if clear != 0 {
                    (clear as *mut i32).write_volatile(0);
                    self.wake_futex(clear, usize::MAX);
                }
            }

            if is_main_thread {
                foundation::kfn::kexit(exit_code);
            }

            self.yield_now();
            0
        } else {
            foundation::kfn::kexit(exit_code);
        }
    }
}
