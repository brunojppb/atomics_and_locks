use std::sync::atomic::{AtomicU8, Ordering};
use std::{cell::UnsafeCell, mem::MaybeUninit};

const EMPTY: u8 = 0;
const WRITING: u8 = 1;
const READY: u8 = 2;
const READING: u8 = 3;

pub struct OneShotChannel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    state: AtomicU8,
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
            state: AtomicU8::new(EMPTY),
        }
    }

    /// # Panics when trying to send more than one message at once
    pub fn send(&self, message: T) {
        if self
            .state
            .compare_exchange(EMPTY, WRITING, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            panic!("A message is already being sent! Can't send more than one message at once!");
        }
        unsafe { (*self.message.get()).write(message) };
        self.state.store(READY, Ordering::Release);
    }

    pub fn is_ready(&self) -> bool {
        self.state.load(Ordering::Relaxed) == READY
    }

    /// Panics if no message is available yet,
    /// or if the message was already consumed.
    /// Always use is_ready() first.
    pub fn receive(&self) -> T {
        if self
            .state
            .compare_exchange(READY, READING, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            panic!("No message available!");
        }

        // Safety: We've just cheked (and reset) the ready flag,
        // so at this point we are guaranteed to have a value ready
        unsafe {
            let v = (*self.message.get()).assume_init_read();
            self.state.store(EMPTY, Ordering::Release);
            v
        }
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
        println!("trying to drop OneShotChannel");
        if *self.state.get_mut() == READY {
            println!("Dropping OneShot channel");
            unsafe { self.message.get_mut().assume_init_drop() }
        } else {
            println!(
                "Only needs to drop if holding some value. UnsafeCell holds no value. moving on..."
            );
        }
    }
}
