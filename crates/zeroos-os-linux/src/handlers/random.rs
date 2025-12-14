use foundation::kfn;
use libc;

pub fn sys_getrandom(buf: usize, buflen: usize, _flags: usize) -> isize {
    if buflen == 0 {
        return 0;
    }
    if buf == 0 {
        return -(libc::EFAULT as isize);
    }
    unsafe { kfn::random::krandom(buf as *mut u8, buflen) }
}
