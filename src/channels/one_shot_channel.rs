use std::sync::atomic::Ordering;
use std::{cell::UnsafeCell, mem::MaybeUninit, sync::atomic::AtomicBool};

pub struct OneShotChannel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
    in_use: AtomicBool,
}

// This allows us to make our UnsafeCell happy
// to share data across threads and by design
// UnsafeCell does not implement Sync to prevent
// data races across threads.
unsafe impl<T> Sync for OneShotChannel<T> where T: Send {}

impl<T> OneShotChannel<T> {
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
            in_use: AtomicBool::new(false),
        }
    }

    /// # Panics when trying to send more than one message at once
    pub fn send(&self, message: T) {
        if self.in_use.swap(true, Ordering::Relaxed) {
            panic!("A message is already being sent! Can't send more than one message at once!");
        }
        unsafe { (*self.message.get()).write(message) };
        self.ready.store(true, Ordering::Release);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    /// Panics if no message is available yet,
    /// or if the message was already consumed.
    /// Always use is_ready() first.
    pub fn receive(&self) -> T {
        if !self.ready.swap(false, Ordering::Acquire) {
            panic!("No message available!");
        }

        // Safety: We've just cheked (and reset) the ready flag,
        // so at this point we are guaranteed to have a value ready
        unsafe { (*self.message.get()).assume_init_read() }
    }
}

///
/// We now have a fully safe interface, though there is still
///  one problem left. The last remaining issue occurs
/// when sending a message that’s never
/// received: it will never be dropped. While
/// this does not result in undefined behavior and
/// is allowed in safe code, it’s definitely something
/// to avoid.
/// Since we reset the ready flag in the receive method,
/// fixing this is easy: the ready flag indicates whether
/// there’s a not-yet-received message in the cell that
/// needs to be dropped.
///
/// In the Drop implementation of our Channel,
/// we don’t need to use an atomic operation to check
/// the atomic ready flag, because an object can only
/// be dropped if it is fully owned by whichever thread
/// is dropping it, with no outstanding borrows.
/// This means we can use the AtomicBool::get_mut method,
/// which takes an exclusive reference (&mut self), proving
/// that atomic access is unnecessary. The same holds for
/// UnsafeCell, through UnsafeCell::get_mut.
impl<T> Drop for OneShotChannel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            println!("Dropping OneShot channel");
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}
