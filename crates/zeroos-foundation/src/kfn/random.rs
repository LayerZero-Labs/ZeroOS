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

/// Macro to implement KRandom trait for integer types.
/// Reduces boilerplate from 12 separate implementations to a single macro invocation.
macro_rules! impl_krandom {
    ($($t:ty),* $(,)?) => {
        $(
            impl KRandom for $t {
                #[inline]
                fn random() -> Self {
                    let mut val: $t = 0;
                    unsafe {
                        krandom(
                            &mut val as *mut $t as *mut u8,
                            core::mem::size_of::<$t>(),
                        )
                    };
                    val
                }
            }
        )*
    };
}

impl_krandom!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
