use std::sync::atomic::AtomicPtr;

#[derive(Debug)]
pub struct Data {
    pub value: usize,
}

pub fn get_data() -> &'static Data {
    static PTR: AtomicPtr<Data> = AtomicPtr::new(std::ptr::null_mut());

    let mut p = PTR.load(std::sync::atomic::Ordering::Acquire);

    if p.is_null() {
        p = Box::into_raw(Box::new(Data { value: 1 }));

        if let Err(e) = PTR.compare_exchange(
            std::ptr::null_mut(),
            p,
            std::sync::atomic::Ordering::Release,
            std::sync::atomic::Ordering::Acquire,
        ) {
            // Safety: p comes from the instance we created right above
            // and wan't shared across any other thread
            drop(unsafe { Box::from_raw(p) });
            p = e;
        }
    }

    unsafe { &*p }
}
