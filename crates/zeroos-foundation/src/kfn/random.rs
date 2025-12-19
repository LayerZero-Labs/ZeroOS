use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "random")] {
        #[inline]
        pub fn kinit(seed: u64) {
            unsafe { (crate::KERNEL.random.init)(seed) }
        }

        #[inline]
        /// # Safety
        /// `buf` must be valid for writes of `len` bytes.
        pub unsafe fn krandom(buf: *mut u8, len: usize) -> isize {
            (crate::KERNEL.random.fill_bytes)(buf, len)
        }
    } else {
        #[inline]
        #[allow(dead_code)]
        pub fn kinit(_seed: u64) {}

        #[inline]
        #[allow(dead_code)]
        /// # Safety
        /// `buf` is not used in the stub implementation.
        pub unsafe fn krandom(_buf: *mut u8, _len: usize) -> isize {
            -1
        }
    }
}

#[allow(dead_code)]
pub trait KRandom: Sized {
    fn random() -> Self;
}

impl KRandom for u8 {
    #[inline]
    fn random() -> Self {
        let mut val = 0u8;
        unsafe { krandom(&mut val as *mut u8, 1) };
        val
    }
}

impl KRandom for u16 {
    #[inline]
    fn random() -> Self {
        let mut val = 0u16;
        unsafe { krandom(&mut val as *mut u16 as *mut u8, 2) };
        val
    }
}

impl KRandom for u32 {
    #[inline]
    fn random() -> Self {
        let mut val = 0u32;
        unsafe { krandom(&mut val as *mut u32 as *mut u8, 4) };
        val
    }
}

impl KRandom for u64 {
    #[inline]
    fn random() -> Self {
        let mut val = 0u64;
        unsafe { krandom(&mut val as *mut u64 as *mut u8, 8) };
        val
    }
}

impl KRandom for u128 {
    #[inline]
    fn random() -> Self {
        let mut val = 0u128;
        unsafe { krandom(&mut val as *mut u128 as *mut u8, 16) };
        val
    }
}

impl KRandom for usize {
    #[inline]
    fn random() -> Self {
        let mut val = 0usize;
        unsafe {
            krandom(
                &mut val as *mut usize as *mut u8,
                core::mem::size_of::<usize>(),
            )
        };
        val
    }
}

impl KRandom for i8 {
    #[inline]
    fn random() -> Self {
        let mut val = 0i8;
        unsafe { krandom(&mut val as *mut i8 as *mut u8, 1) };
        val
    }
}

impl KRandom for i16 {
    #[inline]
    fn random() -> Self {
        let mut val = 0i16;
        unsafe { krandom(&mut val as *mut i16 as *mut u8, 2) };
        val
    }
}

impl KRandom for i32 {
    #[inline]
    fn random() -> Self {
        let mut val = 0i32;
        unsafe { krandom(&mut val as *mut i32 as *mut u8, 4) };
        val
    }
}

impl KRandom for i64 {
    #[inline]
    fn random() -> Self {
        let mut val = 0i64;
        unsafe { krandom(&mut val as *mut i64 as *mut u8, 8) };
        val
    }
}

impl KRandom for i128 {
    #[inline]
    fn random() -> Self {
        let mut val = 0i128;
        unsafe { krandom(&mut val as *mut i128 as *mut u8, 16) };
        val
    }
}

impl KRandom for isize {
    #[inline]
    fn random() -> Self {
        let mut val = 0isize;
        unsafe {
            krandom(
                &mut val as *mut isize as *mut u8,
                core::mem::size_of::<isize>(),
            )
        };
        val
    }
}
