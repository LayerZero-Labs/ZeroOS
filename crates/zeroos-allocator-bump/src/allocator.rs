use core::alloc::Layout;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)]
extern crate alloc;

pub(crate) struct BumpAllocator {
    next: AtomicUsize,

    end: AtomicUsize,
}

impl BumpAllocator {
    pub(crate) const fn new() -> Self {
        Self {
            next: AtomicUsize::new(0),
            end: AtomicUsize::new(0),
        }
    }

    pub(crate) fn init(&self, heap_start: usize, heap_size: usize) {
        self.next.store(heap_start, Ordering::SeqCst);
        let end = heap_start.checked_add(heap_size).unwrap_or(heap_start);
        self.end.store(end, Ordering::SeqCst);
    }

    pub(crate) fn alloc(&self, layout: Layout) -> *mut u8 {
        loop {
            let current = self.next.load(Ordering::Acquire);

            // Align the allocation
            let Some(aligned) = align_up(current, layout.align()) else {
                return ptr::null_mut();
            };
            let new_next = aligned.saturating_add(layout.size());

            if new_next > self.end.load(Ordering::Acquire) {
                return ptr::null_mut();
            }

            if self
                .next
                .compare_exchange(current, new_next, Ordering::Release, Ordering::Acquire)
                .is_ok()
            {
                return aligned as *mut u8;
            }
        }
    }

    #[allow(dead_code)]
    pub unsafe fn reset(&self) {
        let end = self.end.load(Ordering::Acquire);
        let capacity = self.get_capacity();
        let start = end.saturating_sub(capacity);
        self.next.store(start, Ordering::Release);
    }

    #[allow(dead_code)]
    fn get_capacity(&self) -> usize {
        let end = self.end.load(Ordering::Acquire);
        let next = self.next.load(Ordering::Acquire);
        end.saturating_sub(next)
    }
}

/// Align value up to the given alignment.
/// - `value`: Value to align
/// - `align`: Alignment (must be power of 2)
#[inline]
fn align_up(value: usize, align: usize) -> Option<usize> {
    if align == 0 {
        return None;
    }
    value.checked_add(align - 1).map(|v| v & !(align - 1))
}

pub(crate) static ALLOCATOR: BumpAllocator = BumpAllocator::new();

pub(crate) fn init(heap_start: usize, heap_size: usize) {
    ALLOCATOR.init(heap_start, heap_size);
}

pub(crate) fn alloc(layout: Layout) -> *mut u8 {
    ALLOCATOR.alloc(layout)
}

pub(crate) fn dealloc(_ptr: *mut u8, _layout: Layout) {}

pub(crate) fn realloc(ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
    if ptr.is_null() {
        let new_layout = match Layout::from_size_align(new_size, old_layout.align()) {
            Ok(l) => l,
            Err(_) => return ptr::null_mut(),
        };
        return alloc(new_layout);
    }

    if new_size == 0 {
        return ptr::null_mut();
    }

    let new_layout = match Layout::from_size_align(new_size, old_layout.align()) {
        Ok(l) => l,
        Err(_) => return ptr::null_mut(),
    };

    let new_ptr = alloc(new_layout);
    if !new_ptr.is_null() {
        let copy_size = old_layout.size().min(new_size);
        unsafe {
            ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
        }
    }
    new_ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_alloc() {
        const HEAP_SIZE: usize = 1024 * 1024;
        let mut heap_mem = alloc::vec![0u8; HEAP_SIZE];
        let heap_start = heap_mem.as_mut_ptr() as usize;

        init(heap_start, HEAP_SIZE);

        let layout1 = Layout::from_size_align(128, 8).unwrap();
        let ptr1 = alloc(layout1);
        assert!(!ptr1.is_null());

        let layout2 = Layout::from_size_align(256, 16).unwrap();
        let ptr2 = alloc(layout2);
        assert!(!ptr2.is_null());

        assert_ne!(ptr1, ptr2);

        assert!(ptr2 as usize >= ptr1 as usize + 128);
    }

    #[test]
    fn test_bump_out_of_memory() {
        const HEAP_SIZE: usize = 1024;
        let mut heap_mem = alloc::vec![0u8; HEAP_SIZE];
        let heap_start = heap_mem.as_mut_ptr() as usize;

        init(heap_start, HEAP_SIZE);

        let layout = Layout::from_size_align(2048, 8).unwrap();
        let ptr = alloc(layout);
        assert!(ptr.is_null());
    }

    #[test]
    fn test_alignment() {
        const HEAP_SIZE: usize = 1024 * 1024;
        let mut heap_mem = alloc::vec![0u8; HEAP_SIZE];
        let heap_start = heap_mem.as_mut_ptr() as usize;

        init(heap_start, HEAP_SIZE);

        for align in [8, 16, 32, 64, 128, 256].iter() {
            let layout = Layout::from_size_align(64, *align).unwrap();
            let ptr = alloc(layout);
            assert!(!ptr.is_null());
            assert_eq!(ptr as usize % align, 0, "Alignment {} failed", align);
        }
    }
}
