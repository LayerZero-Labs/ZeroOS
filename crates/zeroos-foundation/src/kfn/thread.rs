//! Per-thread/kernel-stack primitives.

/// Per-thread state stored at the base of the kernel stack.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ThreadAnchor {
    /// Kernel stack base (low address).
    pub kstack_base: usize,
    /// Kernel stack size in bytes.
    pub kstack_size: usize,
    /// Optional pointer to a scheduler/TCB structure.
    pub task_ptr: usize,

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

    /// Cached trap-frame address (computed at kstack allocation).
    pub trap_frame_addr: usize,
}

#[inline(always)]
fn compute_trap_frame_addr(
    kstack_base: usize,
    kstack_size: usize,
    trap_frame_size: usize,
    trap_frame_align: usize,
) -> usize {
    let top = match kstack_base.checked_add(kstack_size) {
        Some(v) => v,
        None => {
            debug::writeln!(
                "[THREAD] compute_trap_frame_addr overflow: kstack_base=0x{:x} kstack_size=0x{:x}",
                kstack_base,
                kstack_size
            );
            panic!("compute_trap_frame_addr: kstack_base + kstack_size overflow");
        }
    };

    let tf_size = trap_frame_size;
    let tf_align = trap_frame_align;
    assert!(tf_size != 0);
    assert!(tf_align.is_power_of_two());

    match top.checked_sub(tf_size) {
        Some(v) => v & !(tf_align - 1),
        None => {
            debug::writeln!(
                "[THREAD] compute_trap_frame_addr underflow: top=0x{:x} tf_size=0x{:x} kstack_base=0x{:x} kstack_size=0x{:x}",
                top,
                tf_size,
                kstack_base,
                kstack_size
            );
            panic!("compute_trap_frame_addr: kstack_top < trap_frame_size");
        }
    }
}

/// Allocate a kernel stack and initialize its `ThreadAnchor`.
///
/// Returns the anchor pointer (stack base), or null on failure.
#[inline]
pub fn kalloc_kstack(
    kstack_size: usize,
    trap_frame_size: usize,
    trap_frame_align: usize,
) -> *mut ThreadAnchor {
    assert!(kstack_size.is_power_of_two());

    let base = crate::kfn::memory::kmalloc_aligned(kstack_size, kstack_size);
    if base.is_null() {
        return core::ptr::null_mut();
    }
    let anchor_ptr = base as *mut ThreadAnchor;
    let tf_addr = compute_trap_frame_addr(
        base as usize,
        kstack_size,
        trap_frame_size,
        trap_frame_align,
    );
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
                trap_frame_addr: tf_addr,
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

    let tf = a.trap_frame_addr;
    if tf == 0 {
        debug::writeln!(
            "[THREAD] ktrap_frame_addr missing: anchor=0x{:x} kstack_base=0x{:x} kstack_size=0x{:x}",
            anchor_ptr as usize,
            a.kstack_base,
            a.kstack_size
        );
        panic!("ktrap_frame_addr: trap_frame_addr is 0");
    }

    debug_assert!({
        let top = a.kstack_base.wrapping_add(a.kstack_size);
        tf >= a.kstack_base && tf <= top
    });

    tf
}
