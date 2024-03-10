use std::thread;

use atomics_and_locks::{channels::one_shot_channel::OneShotChannel, locks::spin_lock::SpinLock};

fn main() {
    spin_lock();
    one_shot_channel();
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
