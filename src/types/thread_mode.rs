use std::ops::{Deref, DerefMut};

pub trait Lockable {
    type Guard<'a>: Deref + DerefMut
    where
        Self: 'a;

    fn context_lock(&self) -> Self::Guard<'_>;
}

#[cfg(feature = "serial")]
mod lockable_impl {
    use super::*;

    impl<T> Lockable for T {
        type Guard<'a> = &'a mut T where Self: 'a;

        fn context_lock(&self) -> Self::Guard<'_> {
            // SAFETY: This is safe because we're in single-threaded mode
            unsafe { &mut *(self as *const T as *mut T) }
        }
    }
}

#[cfg(feature = "parallel")]
mod lockable_impl {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    impl<T> Lockable for Mutex<T> {
        type Guard<'a> = MutexGuard<'a, T> where Self: 'a;

        fn context_lock(&self) -> Self::Guard<'_> {
            self.lock().unwrap()
        }
    }
}
