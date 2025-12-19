use core::cell::UnsafeCell;

pub struct GlobalCell<T>(UnsafeCell<T>);

unsafe impl<T> Sync for GlobalCell<T> {}

impl<T> GlobalCell<T> {
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    #[inline(always)]
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(unsafe { &*self.0.get() })
    }

    #[inline(always)]
    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(unsafe { &mut *self.0.get() })
    }
}

pub struct GlobalOption<T>(GlobalCell<Option<T>>);

impl<T> GlobalOption<T> {
    pub const fn none() -> Self {
        Self(GlobalCell::new(None))
    }

    #[inline(always)]
    pub fn with_some<R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.0.with(|slot| slot.as_ref().map(f))
    }

    #[inline(always)]
    pub fn set(&self, value: T) {
        self.0.with_mut(|slot| *slot = Some(value));
    }

    #[inline(always)]
    pub fn is_some(&self) -> bool {
        self.0.with(|slot| slot.is_some())
    }

    #[inline(always)]
    pub fn with_some_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        self.0.with_mut(|slot| slot.as_mut().map(f))
    }
}
