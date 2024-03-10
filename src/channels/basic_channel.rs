use std::{
    collections::VecDeque,
    sync::{Condvar, Mutex},
};

/// Basic implementation of a channel
/// that can send and receive messages across
/// threads (similar to multiple-produces, multiple-consumers (mpmc) channels?)
/// But it has a big downsite:
/// - It
pub struct BasicChannel<T> {
    queue: Mutex<VecDeque<T>>,
    item_ready: Condvar,
}

impl<T> BasicChannel<T> {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            item_ready: Condvar::new(),
        }
    }

    pub fn send(&self, message: T) {
        self.queue.lock().unwrap().push_back(message);
        self.item_ready.notify_one();
    }

    pub fn receive(&self) -> T {
        let mut guard = self.queue.lock().unwrap();
        loop {
            if let Some(message) = guard.pop_front() {
                return message;
            }
            guard = self.item_ready.wait(guard).unwrap();
        }
    }
}

impl<T> Default for BasicChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}
