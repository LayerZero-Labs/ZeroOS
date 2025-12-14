use crate::thread::{apply_thread_ctx_to_frame, sync_thread_ctx_from_frame};
use crate::thread::{ThreadControlBlock, ThreadState, Tid};
use alloc::boxed::Box;
use core::ptr::NonNull;
use foundation::utils::GlobalOption;
use foundation::{ArchContext, FramePointerContext};
use libc;

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

    pub fn init() {
        SCHEDULER.set(Scheduler::new());
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
                (*tcb.as_ptr()).thread_ctx.set_return_value(0);
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
    }

    pub fn wait_on_addr(&mut self, addr: usize, expected: i32) -> isize {
        let actual = unsafe { core::ptr::read_volatile(addr as *const i32) };
        if actual != expected {
            if let Some(tcb) = self.current_thread() {
                unsafe {
                    (*tcb.as_ptr())
                        .thread_ctx
                        .set_return_value((-(libc::EAGAIN as isize)) as usize);
                }
            }
            return -(libc::EAGAIN as isize);
        }

        if self.thread_count() <= 1 {
            if let Some(tcb) = self.current_thread() {
                unsafe {
                    (*tcb.as_ptr())
                        .thread_ctx
                        .set_return_value((-(libc::EDEADLK as isize)) as usize);
                }
            }
            return -(libc::EDEADLK as isize);
        }

        if let Some(tcb) = self.current_thread() {
            unsafe {
                (*tcb.as_ptr()).thread_ctx.set_return_value(0);
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
                (*tcb.as_ptr()).thread_ctx.set_return_value(ret);
            }
        }
        ret
    }

    pub fn spawn_thread(
        &mut self,
        parent_context: crate::context::Context,
        stack: usize,
        tls: usize,
        clear_child_tid_ptr: usize,
        mepc: usize,
    ) -> isize {
        if self.thread_count == 0 {
            let mut main_tcb = Box::new(crate::thread::ThreadControlBlock::new(
                1,
                parent_context.sp(),
                parent_context.tp(),
                mepc,
            ));
            main_tcb.trap_frame = parent_context;
            sync_thread_ctx_from_frame(&mut main_tcb.thread_ctx, &main_tcb.trap_frame);
            main_tcb.saved_pc = mepc;
            main_tcb.state = crate::thread::ThreadState::Running;

            let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(main_tcb)) };
            self.threads[0] = Some(ptr);
            self.thread_count = 1;
            self.current_index = 0;
            self.next_tid = 2;
        }

        let new_tid = self.next_tid;
        self.next_tid += 1;
        let stack_base = stack & !0xF;

        let mut child_tcb = Box::new(crate::thread::ThreadControlBlock::new(
            new_tid, stack_base, tls, mepc,
        ));

        child_tcb.trap_frame = parent_context;
        sync_thread_ctx_from_frame(&mut child_tcb.thread_ctx, &child_tcb.trap_frame);
        child_tcb.thread_ctx.set_sp(stack_base);
        child_tcb.thread_ctx.set_tp(tls);
        child_tcb.thread_ctx.set_return_value(0);
        apply_thread_ctx_to_frame(&mut child_tcb.trap_frame, &child_tcb.thread_ctx);
        child_tcb.trap_frame.set_frame_pointer(stack_base);

        child_tcb.clear_child_tid = clear_child_tid_ptr;

        let child_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(child_tcb)) };
        if self.thread_count >= MAX_THREADS {
            return -(libc::EPERM as isize);
        }
        self.threads[self.thread_count] = Some(child_ptr);
        self.thread_count += 1;

        if let Some(parent_tcb) = self.current_thread() {
            unsafe {
                (*parent_tcb.as_ptr()).thread_ctx.set_return_value(new_tid);
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
                extern "C" {
                    static mut tohost: u64;
                }
                unsafe {
                    let payload = ((exit_code as u64) << 1) | 1;
                    core::ptr::write_volatile(&raw mut tohost, payload);
                }
                return 0;
            }

            if let Some(next_idx) =
                self.find_next_ready((self.current_index + 1) % self.thread_count)
            {
                if let Some(next_tcb) = self.threads[next_idx] {
                    unsafe {
                        (*next_tcb.as_ptr()).state = ThreadState::Running;
                        self.current_index = next_idx;
                    }
                }
            }
            0
        } else {
            extern "C" {
                static mut tohost: u64;
            }
            unsafe {
                let payload = ((exit_code as u64) << 1) | 1;
                core::ptr::write_volatile(&raw mut tohost, payload);
            }
            0
        }
    }
}
