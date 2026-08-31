use core::alloc::Layout;

use foundation::kfn;
use foundation::utils::GlobalCell;
use libc;

const PAGE_SIZE: usize = 4096;

/// Maximum concurrent anonymous mappings. Each entry is two words; the table
/// is a fixed 16 KiB static so mapping bookkeeping never allocates from the
/// heap it accounts for.
const MAX_MMAP_REGIONS: usize = 1024;

/// Live `(base, page-rounded size)` pairs handed out by [`sys_mmap`];
/// `size == 0` marks a free slot.
///
/// The kernel heap only supports freeing the exact `(pointer, layout)` pairs
/// it allocated, while POSIX allows `munmap` to release part of a mapping,
/// span several mappings, or name a range with no mappings at all. Forwarding
/// such calls to `kfree` corrupts the heap free list, so [`sys_munmap`] only
/// frees a munmap that exactly matches a live entry here.
///
/// The unsynchronized cell is sound for the same reason as the scheduler's
/// own `GlobalOption` state: ZeroOS is single-core with no interrupts and no
/// preemption, and thread scheduling is cooperative, so at most one logical
/// flow of execution touches kernel state at a time.
static MMAP_REGIONS: GlobalCell<[(usize, usize); MAX_MMAP_REGIONS]> =
    GlobalCell::new([(0, 0); MAX_MMAP_REGIONS]);

/// Records a region handed out by [`sys_mmap`]; `false` means the table is
/// full and the caller must not hand the region to the guest.
fn record_region(regions: &mut [(usize, usize)], base: usize, size: usize) -> bool {
    match regions.iter_mut().find(|(_, size)| *size == 0) {
        Some(slot) => {
            *slot = (base, size);
            true
        }
        None => false,
    }
}

/// Clears and reports a live region exactly matching `(base, size)`; `false`
/// means the range is not an exact whole-region unmap and must not reach
/// `kfree`.
fn take_exact_region(regions: &mut [(usize, usize)], base: usize, size: usize) -> bool {
    match regions
        .iter_mut()
        .find(|entry| **entry == (base, size) && size != 0)
    {
        Some(slot) => {
            *slot = (0, 0);
            true
        }
        None => false,
    }
}

pub fn sys_brk(_brk: usize) -> isize {
    -(libc::ENOMEM as isize)
}

pub fn sys_mmap(
    addr: usize,
    len: usize,
    prot: usize,
    flags: usize,
    fd: usize,
    offset: usize,
) -> isize {
    if len == 0 {
        return -(libc::EINVAL as isize);
    }
    let allowed_prot = (libc::PROT_NONE | libc::PROT_READ | libc::PROT_WRITE) as usize;
    if (prot & !allowed_prot) != 0 {
        return -(libc::EINVAL as isize);
    }

    let allowed_flags = (libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_STACK) as usize;
    if (flags & !allowed_flags) != 0 {
        return -(libc::EINVAL as isize);
    }
    if (flags & libc::MAP_PRIVATE as usize) == 0 || (flags & libc::MAP_ANONYMOUS as usize) == 0 {
        return -(libc::EINVAL as isize);
    }
    if addr != 0 || offset != 0 {
        return -(libc::EINVAL as isize);
    }
    if fd != usize::MAX && fd != 0 {
        return -(libc::EINVAL as isize);
    }

    let pages = len.div_ceil(PAGE_SIZE);
    let size = match pages.checked_mul(PAGE_SIZE) {
        Some(s) => s,
        None => return -(libc::EINVAL as isize),
    };
    let layout = match Layout::from_size_align(size, PAGE_SIZE) {
        Ok(l) => l,
        Err(_) => return -(libc::EINVAL as isize),
    };
    let ptr = kfn::memory::kmalloc(layout);
    if ptr.is_null() {
        return -(libc::ENOMEM as isize);
    }
    if !MMAP_REGIONS.with_mut(|regions| record_region(regions, ptr as usize, size)) {
        // Fail fast rather than hand out a mapping whose unmap could never
        // be validated (and therefore never freed).
        kfn::memory::kfree(ptr, layout);
        return -(libc::ENOMEM as isize);
    }
    unsafe {
        core::ptr::write_bytes(ptr, 0, size);
    }
    ptr as isize
}

pub fn sys_munmap(addr: usize, len: usize) -> isize {
    if addr == 0 || len == 0 {
        return -(libc::EINVAL as isize);
    }
    if !addr.is_multiple_of(PAGE_SIZE) {
        return -(libc::EINVAL as isize);
    }
    let pages = len.div_ceil(PAGE_SIZE);
    let size = match pages.checked_mul(PAGE_SIZE) {
        Some(s) => s,
        None => return -(libc::EINVAL as isize),
    };
    let layout = match Layout::from_size_align(size, PAGE_SIZE) {
        Ok(l) => l,
        Err(_) => return -(libc::EINVAL as isize),
    };
    if MMAP_REGIONS.with_mut(|regions| take_exact_region(regions, addr, size)) {
        kfn::memory::kfree(addr as *mut u8, layout);
        return 0;
    }
    // POSIX allows partial, repeated, spanning, and no-mapping munmaps; the
    // kernel heap cannot release part of a kmalloc region, so keep the pages
    // mapped (a bounded leak) and report success.
    0
}

pub fn sys_mprotect(addr: usize, len: usize, prot: usize) -> isize {
    if addr == 0 || len == 0 {
        return -(libc::EINVAL as isize);
    }
    if !addr.is_multiple_of(PAGE_SIZE) {
        return -(libc::EINVAL as isize);
    }
    let allowed_prot = (libc::PROT_NONE | libc::PROT_READ | libc::PROT_WRITE) as usize;
    if (prot & !allowed_prot) != 0 {
        return -(libc::EINVAL as isize);
    }
    0
}

#[cfg(test)]
mod tests {
    use super::{record_region, take_exact_region, MAX_MMAP_REGIONS, PAGE_SIZE};

    #[test]
    fn exact_unmap_matches_once() {
        let mut regions = [(0usize, 0usize); 4];
        assert!(record_region(&mut regions, 0x8000_0000, 16 * PAGE_SIZE));
        assert!(take_exact_region(&mut regions, 0x8000_0000, 16 * PAGE_SIZE));
        // Repeated unmap of the released range must not match again.
        assert!(!take_exact_region(
            &mut regions,
            0x8000_0000,
            16 * PAGE_SIZE
        ));
    }

    #[test]
    fn non_exact_unmaps_do_not_match() {
        let mut regions = [(0usize, 0usize); 4];
        assert!(record_region(&mut regions, 0x8000_0000, 16 * PAGE_SIZE));
        // Partial (tail), interior, spanning, and unknown ranges.
        assert!(!take_exact_region(
            &mut regions,
            0x8000_0000 + 8 * PAGE_SIZE,
            8 * PAGE_SIZE
        ));
        assert!(!take_exact_region(&mut regions, 0x8000_0000, 8 * PAGE_SIZE));
        assert!(!take_exact_region(
            &mut regions,
            0x8000_0000,
            32 * PAGE_SIZE
        ));
        assert!(!take_exact_region(
            &mut regions,
            0x9000_0000,
            16 * PAGE_SIZE
        ));
        // The region stays live and its exact unmap still matches.
        assert!(take_exact_region(&mut regions, 0x8000_0000, 16 * PAGE_SIZE));
    }

    #[test]
    fn free_slot_sentinel_never_matches() {
        let mut regions = [(0usize, 0usize); 4];
        assert!(!take_exact_region(&mut regions, 0, 0));
    }

    #[test]
    fn full_table_refuses_and_slots_are_reused() {
        let mut regions = [(0usize, 0usize); MAX_MMAP_REGIONS];
        for i in 0..MAX_MMAP_REGIONS {
            assert!(record_region(&mut regions, (i + 1) * PAGE_SIZE, PAGE_SIZE));
        }
        assert!(!record_region(&mut regions, usize::MAX & !0xfff, PAGE_SIZE));
        assert!(take_exact_region(&mut regions, PAGE_SIZE, PAGE_SIZE));
        assert!(record_region(&mut regions, 42 * PAGE_SIZE, 2 * PAGE_SIZE));
    }
}
