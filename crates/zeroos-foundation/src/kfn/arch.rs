use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "arch")] {
        /// Switch from one thread context to another.
        ///
        /// # Safety
        /// Pointers must be valid and point to architecture-specific thread contexts.
        #[inline]
        pub unsafe fn kswitch_to(old_ctx_ptr: *mut u8, new_ctx_ptr: *const u8) {
            (crate::KERNEL.arch.switch_to)(old_ctx_ptr, new_ctx_ptr)
        }

        #[inline]
        pub fn kthread_ctx_size() -> usize {
            unsafe { (crate::KERNEL.arch.thread_ctx_size)() }
        }

        #[inline]
        pub fn kthread_ctx_align() -> usize {
            unsafe { (crate::KERNEL.arch.thread_ctx_align)() }
        }

        #[inline]
        pub fn ktrap_frame_size() -> usize {
            unsafe { (crate::KERNEL.arch.trap_frame_size)() }
        }

        #[inline]
        pub fn ktrap_frame_align() -> usize {
            unsafe { (crate::KERNEL.arch.trap_frame_align)() }
        }

        /// Initialize a thread context.
        ///
        /// # Safety
        /// `ctx_ptr` must point to a valid memory region for the thread context.
        #[inline]
        pub unsafe fn kthread_ctx_init(ctx_ptr: *mut u8, anchor: usize, kstack_top: usize) {
            (crate::KERNEL.arch.thread_ctx_init)(ctx_ptr, anchor, kstack_top)
        }

        /// Set the stack pointer in a thread context.
        ///
        /// # Safety
        /// `ctx_ptr` must point to a valid thread context.
        #[inline]
        pub unsafe fn kthread_ctx_set_sp(ctx_ptr: *mut u8, sp: usize) {
            (crate::KERNEL.arch.thread_ctx_set_sp)(ctx_ptr, sp)
        }

        /// Set the thread pointer in a thread context.
        ///
        /// # Safety
        /// `ctx_ptr` must point to a valid thread context.
        #[inline]
        pub unsafe fn kthread_ctx_set_tp(ctx_ptr: *mut u8, tp: usize) {
            (crate::KERNEL.arch.thread_ctx_set_tp)(ctx_ptr, tp)
        }

        /// Set the return address in a thread context.
        ///
        /// # Safety
        /// `ctx_ptr` must point to a valid thread context.
        #[inline]
        pub unsafe fn kthread_ctx_set_ra(ctx_ptr: *mut u8, ra: usize) {
            (crate::KERNEL.arch.thread_ctx_set_ra)(ctx_ptr, ra)
        }

        /// Set the return value in a thread context.
        ///
        /// # Safety
        /// `ctx_ptr` must point to a valid thread context.
        #[inline]
        pub unsafe fn kthread_ctx_set_retval(ctx_ptr: *mut u8, val: usize) {
            (crate::KERNEL.arch.thread_ctx_set_retval)(ctx_ptr, val)
        }

        #[inline]
        pub fn kret_from_fork() -> usize {
            unsafe { (crate::KERNEL.arch.ret_from_fork)() }
        }

        /// Clone a trap frame.
        ///
        /// # Safety
        /// `dst` and `src` must point to valid trap frames.
        #[inline]
        pub unsafe fn ktrap_frame_clone(dst: *mut u8, src: *const u8) {
            (crate::KERNEL.arch.trap_frame_clone)(dst, src)
        }

        /// Initialize a trap frame.
        ///
        /// # Safety
        /// `regs` must point to a valid memory region for a trap frame.
        #[inline]
        pub unsafe fn ktrap_frame_init(regs: *mut u8, user_sp: usize, user_tls: usize, pc: usize) {
            (crate::KERNEL.arch.trap_frame_init)(regs, user_sp, user_tls, pc)
        }

        /// Set the return value in a trap frame.
        ///
        /// # Safety
        /// `regs` must point to a valid trap frame.
        #[inline]
        pub unsafe fn ktrap_frame_set_retval(regs: *mut u8, val: usize) {
            (crate::KERNEL.arch.trap_frame_set_retval)(regs, val)
        }

        /// Set the stack pointer in a trap frame.
        ///
        /// # Safety
        /// `regs` must point to a valid trap frame.
        #[inline]
        pub unsafe fn ktrap_frame_set_sp(regs: *mut u8, sp: usize) {
            (crate::KERNEL.arch.trap_frame_set_sp)(regs, sp)
        }

        /// Set the thread pointer in a trap frame.
        ///
        /// # Safety
        /// `regs` must point to a valid trap frame.
        #[inline]
        pub unsafe fn ktrap_frame_set_tp(regs: *mut u8, tp: usize) {
            (crate::KERNEL.arch.trap_frame_set_tp)(regs, tp)
        }

        #[inline]
        pub fn kcurrent_trap_frame() -> *mut u8 {
            unsafe { (crate::KERNEL.arch.current_trap_frame)() }
        }

        /// Get the program counter from a trap frame.
        ///
        /// # Safety
        /// `regs` must point to a valid trap frame.
        #[inline(always)]
        pub unsafe fn ktrap_frame_get_pc(regs: *const u8) -> usize {
            (crate::KERNEL.arch.trap_frame_get_pc)(regs)
        }

        /// Set the program counter in a trap frame.
        ///
        /// # Safety
        /// `regs` must point to a valid trap frame.
        #[inline(always)]
        pub unsafe fn ktrap_frame_set_pc(regs: *mut u8, pc: usize) {
            (crate::KERNEL.arch.trap_frame_set_pc)(regs, pc)
        }

        /// Get the syscall number from a trap frame.
        ///
        /// # Safety
        /// `regs` must point to a valid trap frame.
        #[inline(always)]
        pub unsafe fn ktrap_frame_get_nr(regs: *const u8) -> usize {
            (crate::KERNEL.arch.trap_frame_get_nr)(regs)
        }

        /// Get a syscall argument from a trap frame.
        ///
        /// # Safety
        /// `regs` must point to a valid trap frame.
        #[inline(always)]
        pub unsafe fn ktrap_frame_get_arg(regs: *const u8, idx: usize) -> usize {
            (crate::KERNEL.arch.trap_frame_get_arg)(regs, idx)
        }

        /// Get the trap cause from a trap frame.
        ///
        /// # Safety
        /// `regs` must point to a valid trap frame.
        #[inline(always)]
        pub unsafe fn ktrap_frame_get_cause(regs: *const u8) -> usize {
            (crate::KERNEL.arch.trap_frame_get_cause)(regs)
        }

        /// Get the fault address from a trap frame.
        ///
        /// # Safety
        /// `regs` must point to a valid trap frame.
        #[inline(always)]
        pub unsafe fn ktrap_frame_get_fault_addr(regs: *const u8) -> usize {
            (crate::KERNEL.arch.trap_frame_get_fault_addr)(regs)
        }
    } else {
        /// Stub implementation of `kswitch_to`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn kswitch_to(_old_ctx_ptr: *mut u8, _new_ctx_ptr: *const u8) {}

        #[inline]
        #[allow(dead_code)]
        pub fn kthread_ctx_size() -> usize {
            0
        }

        #[inline]
        #[allow(dead_code)]
        pub fn kthread_ctx_align() -> usize {
            1
        }

        #[inline]
        #[allow(dead_code)]
        pub fn ktrap_frame_size() -> usize {
            0
        }

        #[inline]
        #[allow(dead_code)]
        pub fn ktrap_frame_align() -> usize {
            1
        }

        /// Stub implementation of `kthread_ctx_init`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn kthread_ctx_init(_ctx_ptr: *mut u8, _anchor: usize, _kstack_top: usize) {}

        /// Stub implementation of `kthread_ctx_set_sp`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn kthread_ctx_set_sp(_ctx_ptr: *mut u8, _sp: usize) {}

        /// Stub implementation of `kthread_ctx_set_tp`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn kthread_ctx_set_tp(_ctx_ptr: *mut u8, _tp: usize) {}

        /// Stub implementation of `kthread_ctx_set_ra`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn kthread_ctx_set_ra(_ctx_ptr: *mut u8, _ra: usize) {}

        /// Stub implementation of `kthread_ctx_set_retval`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn kthread_ctx_set_retval(_ctx_ptr: *mut u8, _val: usize) {}

        #[inline]
        #[allow(dead_code)]
        pub fn kret_from_fork() -> usize {
            0
        }

        /// Stub implementation of `ktrap_frame_clone`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn ktrap_frame_clone(_dst: *mut u8, _src: *const u8) {}

        /// Stub implementation of `ktrap_frame_init`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn ktrap_frame_init(_regs: *mut u8, _user_sp: usize, _user_tls: usize, _pc: usize) {}

        /// Stub implementation of `ktrap_frame_set_retval`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn ktrap_frame_set_retval(_regs: *mut u8, _val: usize) {}

        /// Stub implementation of `ktrap_frame_set_sp`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn ktrap_frame_set_sp(_regs: *mut u8, _sp: usize) {}

        /// Stub implementation of `ktrap_frame_set_tp`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn ktrap_frame_set_tp(_regs: *mut u8, _tp: usize) {}

        #[inline]
        #[allow(dead_code)]
        pub fn kcurrent_trap_frame() -> *mut u8 {
            core::ptr::null_mut()
        }

        /// Stub implementation of `ktrap_frame_get_pc`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn ktrap_frame_get_pc(_regs: *const u8) -> usize {
            0
        }

        /// Stub implementation of `ktrap_frame_set_pc`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn ktrap_frame_set_pc(_regs: *mut u8, _pc: usize) {}

        /// Stub implementation of `ktrap_frame_get_nr`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn ktrap_frame_get_nr(_regs: *const u8) -> usize {
            0
        }

        /// Stub implementation of `ktrap_frame_get_arg`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn ktrap_frame_get_arg(_regs: *const u8, _idx: usize) -> usize {
            0
        }

        /// Stub implementation of `ktrap_frame_get_cause`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn ktrap_frame_get_cause(_regs: *const u8) -> usize {
            0
        }

        /// Stub implementation of `ktrap_frame_get_fault_addr`.
        ///
        /// # Safety
        /// This is a stub and does nothing.
        #[inline]
        #[allow(dead_code)]
        pub unsafe fn ktrap_frame_get_fault_addr(_regs: *const u8) -> usize {
            0
        }
    }
}
