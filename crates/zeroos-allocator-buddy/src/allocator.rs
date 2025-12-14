use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

use buddy_system_allocator::LockedHeap;

// Upstream buddy allocator. The const generic is the max order, i.e. the maximum
// heap size is bounded by \(2^\text{ORDER}\) bytes.
//
// 32 is a common, conservative choice for embedded/kernel heaps.
const ORDER: usize = 32;

pub(crate) static HEAP: LockedHeap<ORDER> = LockedHeap::empty();

pub(crate) fn init(heap_start: usize, heap_size: usize) {
    unsafe {
        HEAP.lock().init(heap_start, heap_size);
    }
}

pub(crate) fn alloc(layout: Layout) -> *mut u8 {
    unsafe { GlobalAlloc::alloc(&HEAP, layout) }
}

pub(crate) fn dealloc(ptr: *mut u8, layout: Layout) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        GlobalAlloc::dealloc(&HEAP, ptr, layout);
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

    unsafe { GlobalAlloc::realloc(&HEAP, ptr, old_layout, new_size) }
}
