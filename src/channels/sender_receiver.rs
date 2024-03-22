use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::{atomic::AtomicBool, Arc},
};

pub struct Sender<T> {
    channel: Arc<Channel<T>>,
}

impl<T> Sender<T> {
    // This never panics. trust me bro
    pub fn send(self, message: T) {
        unsafe {
            (*self.channel.message.get()).write(message);
        };
        self.channel
            .ready
            .store(true, std::sync::atomic::Ordering::Release)
    }
}

pub struct Receiver<T> {
    channel: Arc<Channel<T>>,
}

impl<T> Receiver<T> {
    pub fn is_ready(&self) -> bool {
        self.channel
            .ready
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn receive(self) -> T {
        if !self
            .channel
            .ready
            .swap(false, std::sync::atomic::Ordering::Acquire)
        {
            panic!("No message available!")
        }
        unsafe { (*self.channel.message.get()).assume_init_read() }
    }
}

struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let a = Arc::new(Channel {
        message: UnsafeCell::new(MaybeUninit::uninit()),
        ready: AtomicBool::new(false),
    });
    (Sender { channel: a.clone() }, Receiver { channel: a })
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}
