use std::sync::atomic::Ordering::*;
use std::{ops::Deref, ptr::NonNull, sync::atomic::AtomicUsize};

struct ArcData<T> {
    ref_count: AtomicUsize,
    data: T,
}

pub struct Arc<T> {
    ptr: NonNull<ArcData<T>>,
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        Arc {
            ptr: NonNull::from(Box::leak(Box::new(ArcData {
                ref_count: AtomicUsize::new(1),
                data,
            }))),
        }
    }

    fn data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        // prevent ref count overflow
        if self.data().ref_count.fetch_add(1, Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }
        Arc { ptr: self.ptr }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data().data
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Release) == 1 {
            unsafe { drop(Box::from_raw(self.ptr.as_ptr())) }
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::AtomicUsize;

    use super::Arc;

    static NUM_DROPS: AtomicUsize = AtomicUsize::new(0);

    struct DetectDrop;

    impl Drop for DetectDrop {
        fn drop(&mut self) {
            NUM_DROPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    #[test]
    fn test_drop() {
        let x = Arc::new(("Hello", DetectDrop));
        let y = x.clone();

        // send X to another thread, incrementing the counter
        let t = std::thread::spawn(move || {
            assert_eq!(x.0, "Hello");
        });

        // In parallel, on the current thread, y should still be usable.
        assert_eq!(y.0, "Hello");

        t.join().unwrap();

        // x should be droped by now as it moved to the new thread
        // and after the .join() it's out of scope.
        // but y should stay around, so the shared tuple should not be dropped
        assert_eq!(NUM_DROPS.load(std::sync::atomic::Ordering::Relaxed), 0);

        drop(y);

        // now y should be gone
        // and the underlying tuple should be completely gone
        assert_eq!(NUM_DROPS.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}
