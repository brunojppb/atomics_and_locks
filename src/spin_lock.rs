use std::{
    cell::UnsafeCell,
    hint::spin_loop,
    ops::{Deref, DerefMut},
    sync::atomic::AtomicBool,
};

pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

unsafe impl<T> Sync for SpinLock<T> where T: Send {}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> Guard<T> {
        while self
            .locked
            .compare_exchange_weak(
                false,
                true,
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            spin_loop();
        }
        Guard { lock: self }
    }

    pub fn unlock(&self) {
        self.locked
            .store(false, std::sync::atomic::Ordering::Release)
    }
}

pub struct Guard<'a, T> {
    lock: &'a SpinLock<T>,
}

// Deref works as a proxy here by providing
// a controled unsafe interface access to the actual lock value.
//
// This helps us guarantee that the value is only accessed
// by one thread at a time.
impl<T> Deref for Guard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: The very existence of this Guard
        // guarantees we've exclusively locked the spin lock
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: The very existence of this Guard
        // guarantees we've exclusively locked the spin lock
        unsafe { &mut *self.lock.value.get() }
    }
}

// whenever the guard goes out of scope
// The spin lock will be released and other threads
// can get a hold on the value from now on.
impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        self.lock
            .locked
            .store(false, std::sync::atomic::Ordering::Release)
    }
}
