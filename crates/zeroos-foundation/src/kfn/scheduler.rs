use cfg_if::cfg_if;

#[cfg(feature = "scheduler")]
#[allow(unused_imports)]
pub use crate::kfn::thread::ktrap_frame_addr;
#[cfg(feature = "scheduler")]
#[allow(unused_imports)]
pub use crate::kfn::thread::{kalloc_kstack, ThreadAnchor};

cfg_if! {
    if #[cfg(feature = "scheduler")] {
        #[inline]
        pub fn kinit() -> usize {
            unsafe { (crate::KERNEL.scheduler.init)() }
        }

        #[inline]
        pub fn kspawn_thread(
            stack: usize,
            tls: usize,
            parent_tid_ptr: usize,
            child_tid_ptr: usize,
            clear_child_tid_ptr: usize,
        ) -> isize {
            unsafe {
                (crate::KERNEL.scheduler.spawn_thread)(
                    stack,
                    tls,
                    parent_tid_ptr,
                    child_tid_ptr,
                    clear_child_tid_ptr,
                )
            }
        }

        #[inline]
        pub fn ksched_yield() -> isize {
            unsafe { (crate::KERNEL.scheduler.yield_now)() }
        }

        #[inline]
        pub fn kexit_current(code: i32) -> isize {
            unsafe { (crate::KERNEL.scheduler.exit_current)(code) }
        }

        #[inline]
        pub fn kcurrent_tid() -> usize {
            unsafe { (crate::KERNEL.scheduler.current_tid)() }
        }

        #[inline]
        pub fn kthread_count() -> usize {
            unsafe { (crate::KERNEL.scheduler.thread_count)() }
        }

        #[inline]
        pub fn kwait_on_addr(addr: usize, expected: i32) -> isize {
            unsafe { (crate::KERNEL.scheduler.wait_on_addr)(addr, expected) }
        }

        #[inline]
        pub fn kwake_on_addr(addr: usize, count: usize) -> usize {
            unsafe { (crate::KERNEL.scheduler.wake_on_addr)(addr, count) }
        }

        #[inline]
        pub fn kset_clear_on_exit_addr(addr: usize) -> isize {
            unsafe { (crate::KERNEL.scheduler.set_clear_on_exit_addr)(addr) }
        }
    } else {
        #[inline]
        #[allow(dead_code)]
        pub fn kinit() -> usize {
            0
        }

        #[inline]
        #[allow(dead_code)]
        pub fn kspawn_thread(
            _stack: usize,
            _tls: usize,
            _parent_tid_ptr: usize,
            _child_tid_ptr: usize,
            _clear_child_tid_ptr: usize,
        ) -> isize {
            -1
        }

        #[inline]
        #[allow(dead_code)]
        pub fn ksched_yield() -> isize {
            0
        }

        #[inline]
        #[allow(dead_code)]
        pub fn kexit_current(_code: i32) -> isize {
            0
        }

        #[inline]
        #[allow(dead_code)]
        pub fn kcurrent_tid() -> usize {
            1
        }

        #[inline]
        #[allow(dead_code)]
        pub fn kthread_count() -> usize {
            1
        }

        #[inline]
        #[allow(dead_code)]
        pub fn kwait_on_addr(_addr: usize, _expected: i32) -> isize {
            0
        }

        #[inline]
        #[allow(dead_code)]
        pub fn kwake_on_addr(_addr: usize, _count: usize) -> usize {
            0
        }

        #[inline]
        #[allow(dead_code)]
        pub fn kset_clear_on_exit_addr(_addr: usize) -> isize {
            0
        }
    }
}
