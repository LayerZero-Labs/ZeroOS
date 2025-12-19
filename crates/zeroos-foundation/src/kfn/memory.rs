use core::alloc::Layout;
use core::ptr;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "memory")] {
        #[inline]
        pub fn kmalloc(layout: Layout) -> *mut u8 {
            unsafe { (crate::KERNEL.memory.alloc)(layout) }
        }

        #[inline]
        pub fn kfree(ptr: *mut u8, layout: Layout) {
            unsafe { (crate::KERNEL.memory.dealloc)(ptr, layout) }
        }

        #[inline]
        pub fn krealloc(ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
            unsafe { (crate::KERNEL.memory.realloc)(ptr, old_layout, new_size) }
        }

        #[inline]
        pub fn kinit(heap_start: usize, heap_size: usize) {
            unsafe { (crate::KERNEL.memory.init)(heap_start, heap_size) }
        }
    } else {
        #[inline]
        #[allow(dead_code)]
        pub fn kmalloc(_layout: Layout) -> *mut u8 {
            ptr::null_mut()
        }

        #[inline]
        #[allow(dead_code)]
        pub fn kfree(_ptr: *mut u8, _layout: Layout) {}

        #[inline]
        #[allow(dead_code)]
        pub fn krealloc(_ptr: *mut u8, _old_layout: Layout, _new_size: usize) -> *mut u8 {
            ptr::null_mut()
        }

        #[inline]
        #[allow(dead_code)]
        pub fn kinit(_heap_start: usize, _heap_size: usize) {}
    }
}

#[inline]
pub fn kzalloc(layout: Layout) -> *mut u8 {
    let ptr = kmalloc(layout);
    if !ptr.is_null() {
        unsafe {
            ptr::write_bytes(ptr, 0, layout.size());
        }
    }
    ptr
}

#[no_mangle]
pub extern "C" fn kmalloc_aligned(size: usize, align: usize) -> *mut u8 {
    Layout::from_size_align(size, align)
        .map(kmalloc)
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn kmalloc_size(size: usize) -> *mut u8 {
    kmalloc_aligned(size, core::mem::size_of::<usize>())
}

#[no_mangle]
pub extern "C" fn kzalloc_size(size: usize) -> *mut u8 {
    Layout::from_size_align(size, core::mem::size_of::<usize>())
        .map(kzalloc)
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn kfree_aligned(ptr: *mut u8, size: usize, align: usize) {
    if let Ok(layout) = Layout::from_size_align(size, align) {
        kfree(ptr, layout);
    }
}

#[no_mangle]
pub extern "C" fn kfree_size(ptr: *mut u8, size: usize) {
    kfree_aligned(ptr, size, core::mem::size_of::<usize>());
}

#[no_mangle]
pub extern "C" fn krealloc_size(ptr: *mut u8, old_size: usize, new_size: usize) -> *mut u8 {
    Layout::from_size_align(old_size, core::mem::size_of::<usize>())
        .map(|l| krealloc(ptr, l, new_size))
        .unwrap_or(ptr::null_mut())
}
