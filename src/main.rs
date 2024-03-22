use std::thread;

use atomics_and_locks::{
    channels::{one_shot_channel::OneShotChannel, sender_receiver::channel},
    locks::spin_lock::SpinLock,
};

fn main() {
    spin_lock();
    one_shot_channel();
    send_receiver_channel();
}

fn one_shot_channel() {
    let channel = OneShotChannel::new();
    let t = thread::current();

    thread::scope(|s| {
        s.spawn(|| {
            channel.send("Hi there!");
            t.unpark();
        });

        while !channel.is_ready() {
            println!("Parking main thread...");
            thread::park();
        }

        println!("Unparking...");

        assert_eq!(channel.receive(), "Hi there!");

        channel.send("Test dropping...");
    });
}

fn spin_lock() {
    let spin = SpinLock::new(Vec::new());

    thread::scope(|scope| {
        scope.spawn(|| {
            spin.lock().push(1);
        });

        scope.spawn(|| {
            spin.lock().push(2);
        });
    });

    let list = spin.lock();
    println!("List value: {:?}", list.as_slice());
    assert!(list.as_slice() == [1, 2] || list.as_slice() == [2, 1])
}

fn send_receiver_channel() {
    thread::scope(|s| {
        let (tx, rx) = channel();
        let t = thread::current();
        s.spawn(move || {
            tx.send("Hi there!");
            t.unpark();
        });

        while !rx.is_ready() {
            thread::park();
        }

        assert_eq!(rx.receive(), "Hi there!");
    })
}
