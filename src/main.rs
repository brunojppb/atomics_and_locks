use atomics_and_locks::spin_lock;

fn main() {
    println!("Hello, world!");
    let result = spin_lock::get_data();
    println!("Result: {:?}", result.value);
}
