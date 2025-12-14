#![no_std]

extern "C" {
    fn platform_exit(code: i32) -> !;
}

#[inline(always)]
fn unwind_abort() -> ! {
    unsafe { platform_exit(101) }
}

// `_Unwind_Reason_Code` values (GCC/libunwind ABI).
const _URC_END_OF_STACK: i32 = 5;

#[no_mangle]
pub extern "C" fn _Unwind_Resume(_exception: *mut u8) -> ! {
    unwind_abort()
}

#[no_mangle]
pub extern "C" fn _Unwind_Backtrace(
    _trace_fn: extern "C" fn(*mut u8, *mut u8) -> i32,
    _trace_argument: *mut u8,
) -> i32 {
    _URC_END_OF_STACK
}

#[no_mangle]
pub extern "C" fn _Unwind_GetIP(_context: *mut u8) -> usize {
    unwind_abort()
}

#[no_mangle]
pub extern "C" fn _Unwind_GetIPInfo(_context: *mut u8, _ip_before_insn: *mut i32) -> i32 {
    unwind_abort()
}

#[no_mangle]
pub extern "C" fn _Unwind_GetCFA(_context: *mut u8) -> usize {
    unwind_abort()
}

#[no_mangle]
pub extern "C" fn _Unwind_GetLanguageSpecificData(_context: *mut u8) -> *mut u8 {
    unwind_abort()
}

#[no_mangle]
pub extern "C" fn _Unwind_GetRegionStart(_context: *mut u8) -> usize {
    unwind_abort()
}

#[no_mangle]
pub extern "C" fn _Unwind_GetTextRelBase(_context: *mut u8) -> usize {
    unwind_abort()
}

#[no_mangle]
pub extern "C" fn _Unwind_GetDataRelBase(_context: *mut u8) -> usize {
    unwind_abort()
}

#[no_mangle]
pub extern "C" fn _Unwind_SetGR(_context: *mut u8, _index: i32, _value: usize) {
    unwind_abort()
}

#[no_mangle]
pub extern "C" fn _Unwind_SetIP(_context: *mut u8, _value: usize) {
    unwind_abort()
}

#[no_mangle]
pub extern "C" fn _Unwind_FindEnclosingFunction(_pc: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}
