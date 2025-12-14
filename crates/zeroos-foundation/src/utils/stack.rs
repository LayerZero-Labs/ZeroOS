use core::marker::PhantomData;
use core::mem;
use core::ptr;

/// # Safety
/// Caller must ensure `sp` points to writable memory and respects
pub struct DownwardStack<T> {
    sp: usize,
    _marker: PhantomData<T>,
}

impl<T> DownwardStack<T> {
    /// The initial sp will be aligned to meet architecture-specific ABI requirements:
    #[inline]
    pub fn new(initial_sp: usize) -> Self {
        cfg_if::cfg_if! {
            if #[cfg(any(
                target_arch = "riscv32",
                target_arch = "riscv64",
                target_arch = "x86_64",
                target_arch = "aarch64",
                target_arch = "x86",
            ))] {

                let min_align = 16;
            } else if #[cfg(target_arch = "arm")] {

                let min_align = 8;
            } else {

                let min_align = 2 * mem::size_of::<usize>();
            }
        }

        let align = mem::align_of::<T>().max(min_align);
        let aligned_sp = initial_sp & !(align - 1);
        Self {
            sp: aligned_sp,
            _marker: PhantomData,
        }
    }

    /// # Safety
    /// Caller must ensure there is sufficient stack space below the current sp.
    #[inline]
    pub unsafe fn push(&mut self, value: T) {
        self.sp -= mem::size_of::<T>();
        unsafe {
            ptr::write(self.sp as *mut T, value);
        }
    }

    /// # Safety
    /// Caller must ensure there is a valid value at the current sp.
    #[inline]
    pub unsafe fn pop(&mut self) -> T {
        let value = unsafe { ptr::read(self.sp as *const T) };
        self.sp += mem::size_of::<T>();
        value
    }

    /// # Safety
    /// Caller must ensure the computed address contains a valid value of type T.
    #[inline]
    pub unsafe fn pick(&self, offset: isize) -> T {
        let addr = self.addr_at(offset);
        unsafe { ptr::read(addr as *const T) }
    }

    #[inline]
    pub fn addr_at(&self, offset: isize) -> usize {
        (self.sp as isize + offset * mem::size_of::<T>() as isize) as usize
    }

    #[inline]
    pub fn sp(&self) -> usize {
        self.sp
    }
}
