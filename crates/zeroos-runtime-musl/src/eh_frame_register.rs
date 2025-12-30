//! Register `.eh_frame` with libgcc's unwinder.
//!
//! We do this from `.init_array` so it runs after musl's `__init_libc` (malloc/env are ready)
//! but before `main`, avoiding early-boot allocations/faults.
#![cfg(feature = "backtrace")]

extern "C" {
    // libgcc frame registration API (DWARF2 unwinder)
    fn __register_frame(begin: *const u8);
    // Provided by the linker script (we KEEP .eh_frame and export these).
    static __eh_frame_start: u8;
    static __eh_frame_end: u8;
}

#[no_mangle]
extern "C" fn __zeroos_register_eh_frame() {
    let start = core::ptr::addr_of!(__eh_frame_start) as *const u8;
    let end = core::ptr::addr_of!(__eh_frame_end) as *const u8;
    if start != end {
        unsafe { __register_frame(start) };
    }
}

// Place a pointer to our init function in `.init_array` so musl calls it from `__libc_start_init`.
#[used]
#[link_section = ".init_array"]
static __ZEROOS_EH_FRAME_INIT: extern "C" fn() = __zeroos_register_eh_frame;
