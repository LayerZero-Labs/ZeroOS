//! Trap handling wrappers.

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "trap")] {
        #[inline]
        pub fn ksyscall(
            a0: usize,
            a1: usize,
            a2: usize,
            a3: usize,
            a4: usize,
            a5: usize,
            nr: usize,
        ) -> isize {
            unsafe { (crate::KERNEL.trap.syscall)(a0, a1, a2, a3, a4, a5, nr) }
        }

        #[inline]
        pub fn kexception(code: usize, pc: usize, trap_value: usize) -> Option<usize> {
            unsafe { (crate::KERNEL.trap.exception)(code, pc, trap_value) }
        }

        #[inline]
        pub fn kinterrupt(code: usize) {
            unsafe { (crate::KERNEL.trap.interrupt)(code) }
        }
    } else {
        #[inline]
        #[allow(dead_code)]
        pub fn ksyscall(
            _a0: usize,
            _a1: usize,
            _a2: usize,
            _a3: usize,
            _a4: usize,
            _a5: usize,
            _nr: usize,
        ) -> isize {
            -38 // ENOSYS
        }

        #[inline]
        #[allow(dead_code)]
        pub fn kexception(_code: usize, _pc: usize, _trap_value: usize) -> Option<usize> {
            None
        }

        #[inline]
        #[allow(dead_code)]
        pub fn kinterrupt(_code: usize) {}
    }
}
