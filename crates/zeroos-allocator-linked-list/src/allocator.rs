use core::alloc::Layout;
use core::ptr;
use linked_list_allocator::LockedHeap;

#[cfg(test)]
extern crate alloc;

pub(crate) static HEAP: LockedHeap = LockedHeap::empty();

pub(crate) fn init(heap_start: usize, heap_size: usize) {
    unsafe {
        HEAP.lock().init(heap_start as *mut u8, heap_size);
    }
}

pub(crate) fn alloc(layout: Layout) -> *mut u8 {
    HEAP.lock()
        .allocate_first_fit(layout)
        .map(|nn| nn.as_ptr())
        .unwrap_or(ptr::null_mut())
}

pub(crate) fn dealloc(ptr: *mut u8, layout: Layout) {
    if !ptr.is_null() {
        unsafe {
            HEAP.lock()
                .deallocate(ptr::NonNull::new_unchecked(ptr), layout);
        }
    }
}

pub(crate) fn realloc(ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
    if ptr.is_null() {
        let new_layout = match Layout::from_size_align(new_size, old_layout.align()) {
            Ok(l) => l,
            Err(_) => return ptr::null_mut(),
        };
        return alloc(new_layout);
    }

    if new_size == 0 {
        dealloc(ptr, old_layout);
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
        dealloc(ptr, old_layout);
    }
    new_ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_dealloc() {
        const HEAP_SIZE: usize = 1024 * 1024;
        let mut heap_mem = alloc::vec![0u8; HEAP_SIZE];
        let heap_start = heap_mem.as_mut_ptr() as usize;

        init(heap_start, HEAP_SIZE);

        let layout = Layout::from_size_align(128, 8).unwrap();
        let ptr = alloc(layout);
        assert!(!ptr.is_null());

        dealloc(ptr, layout);
    }

    #[test]
    fn test_realloc() {
        const HEAP_SIZE: usize = 1024 * 1024;
        let mut heap_mem = alloc::vec![0u8; HEAP_SIZE];
        let heap_start = heap_mem.as_mut_ptr() as usize;

        init(heap_start, HEAP_SIZE);

        let layout = Layout::from_size_align(128, 8).unwrap();
        let ptr = alloc(layout);
        assert!(!ptr.is_null());

        unsafe {
            ptr::write_bytes(ptr, 0x42, 64);
        }

        let new_ptr = realloc(ptr, layout, 256);
        assert!(!new_ptr.is_null());

        unsafe {
            for i in 0..64 {
                assert_eq!(*new_ptr.add(i), 0x42);
            }
        }

        let new_layout = Layout::from_size_align(256, 8).unwrap();
        dealloc(new_ptr, new_layout);
    }
}
