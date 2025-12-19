//! Low-level per-thread/kernel-stack primitives.
//!
//! Independent of the scheduler API; usable for trap entry/exit without a scheduler.

/// Per-thread state stored at the base of the kernel stack.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ThreadAnchor {
    pub kstack_base: usize, // Stack base (low address)
    pub kstack_size: usize, // Stack size in bytes
    pub task_ptr: usize,    // Opaque pointer to TCB (optional)

    /// Current/last saved kernel stack pointer.
    /// Tracks the kernel stack position across trap entry/exit and context switches.
    pub kernel_sp: usize,

    /// Saved user stack pointer.
    /// Stashes the userspace SP when trapping into the kernel.
    pub user_sp: usize,

    /// Architecture-specific stash slots for register preservation during trap entry/exit.
    pub stash0: usize,
    pub stash1: usize,
    pub stash2: usize,
}

/// Allocate a kernel stack and initialize a `ThreadAnchor` at its base.
///
/// Returns a pointer to the anchor (stack base), or null on failure.
#[inline]
pub fn kalloc_kstack(kstack_size: usize) -> *mut ThreadAnchor {
    assert!(kstack_size.is_power_of_two());

    let base = crate::kfn::memory::kmalloc_aligned(kstack_size, kstack_size);
    if base.is_null() {
        return core::ptr::null_mut();
    }
    let anchor_ptr = base as *mut ThreadAnchor;
    unsafe {
        core::ptr::write(
            anchor_ptr,
            ThreadAnchor {
                kstack_base: base as usize,
                kstack_size,
                task_ptr: 0,
                kernel_sp: (base as usize) + kstack_size,
                user_sp: 0,
                stash0: 0,
                stash1: 0,
                stash2: 0,
            },
        );
    }
    anchor_ptr
}

/// Compute the trap-frame address at the top of the thread's kernel stack.
///
/// # Safety
/// `anchor_ptr` must point to a valid `ThreadAnchor` structure.
#[inline(always)]
pub unsafe fn ktrap_frame_addr(anchor_ptr: *const ThreadAnchor) -> usize {
    assert!(!anchor_ptr.is_null());
    let a = &*anchor_ptr;
    let top = a.kstack_base + a.kstack_size;
    let tf_size = crate::kfn::arch::ktrap_frame_size();
    let tf_align = crate::kfn::arch::ktrap_frame_align();
    assert!(tf_size != 0);
    assert!(tf_align.is_power_of_two());
    let tf = top - tf_size;
    tf & !(tf_align - 1)
}
