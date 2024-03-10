use std::thread;

use atomics_and_locks::locks::spin_lock::SpinLock;

fn main() {
    spin_lock();
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
