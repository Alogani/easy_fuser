use std::ops::{Deref, DerefMut};

pub trait SafeBorrowable {
    type Guard<'a>: Deref + DerefMut
    where
        Self: 'a;

    fn safe_borrow_mut(&self) -> Self::Guard<'_>;
}

#[cfg(feature = "serial")]
mod safe_borrowable_impl {
    use super::*;

    use std::cell::{RefCell, RefMut};

    impl<T> SafeBorrowable for RefCell<T> {
        type Guard<'a> = RefMut<'a, T> where Self: 'a;

        fn safe_borrow_mut(&self) -> Self::Guard<'_> {
            self.borrow_mut()
        }
    }
}

#[cfg(all(feature = "parallel", not(feature = "deadlock_detection")))]
mod safe_borrowable_impl {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    impl<T> SafeBorrowable for Mutex<T> {
        type Guard<'a> = MutexGuard<'a, T> where Self: 'a;

        fn safe_borrow_mut(&self) -> Self::Guard<'_> {
            self.lock().unwrap()
        }
    }
}

#[cfg(all(feature = "parallel", feature = "deadlock_detection"))]
mod safe_borrowable_impl {
    use super::*;
    use parking_lot::{Mutex, MutexGuard};

    impl<T> SafeBorrowable for Mutex<T> {
        type Guard<'a> = MutexGuard<'a, T> where Self: 'a;

        fn safe_borrow_mut(&self) -> Self::Guard<'_> {
            self.lock()
        }
    }
}
