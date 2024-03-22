use std::cell::UnsafeCell;
use std::sync::atomic::{fence, Ordering::*};
use std::{ops::Deref, ptr::NonNull, sync::atomic::AtomicUsize};

struct ArcData<T> {
    /// Number of Arc references
    data_ref_count: AtomicUsize,
    /// Number of Arc and Weak references combined
    alloc_ref_count: AtomicUsize,
    // None if there is only weak pointers left
    data: UnsafeCell<Option<T>>,
}

pub struct Weak<T> {
    ptr: NonNull<ArcData<T>>,
}

pub struct Arc<T> {
    weak: Weak<T>,
}

unsafe impl<T: Send + Sync> Send for Weak<T> {}
unsafe impl<T: Send + Sync> Sync for Weak<T> {}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        Arc {
            weak: Weak {
                ptr: NonNull::from(Box::leak(Box::new(ArcData {
                    data_ref_count: AtomicUsize::new(1),
                    alloc_ref_count: AtomicUsize::new(1),
                    data: UnsafeCell::new(Some(data)),
                }))),
            },
        }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.weak.data().alloc_ref_count.load(Relaxed) == 1 {
            fence(Acquire);
            // Safety: Nothing else can access the data, since
            // there is only one Arc, to which we have exclusive access
            let arc_data = unsafe { arc.weak.ptr.as_mut() };
            let option = arc_data.data.get_mut();
            let data = option.as_mut().unwrap();
            Some(data)
        } else {
            None
        }
    }
}

impl<T> Weak<T> {
    fn data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        if self.data().alloc_ref_count.fetch_add(1, Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }
        Weak { ptr: self.ptr }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        let weak = self.weak.clone();
        if weak.data().data_ref_count.fetch_add(1, Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }
        Arc { weak }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let ptr = self.weak.data().data.get();
        // Safety: Sunce there's an Arc to the data,
        // the data exists adn may be shared.
        unsafe { (*ptr).as_ref().unwrap() }
    }
}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        if self.data().alloc_ref_count.fetch_sub(1, Release) == 1 {
            unsafe { drop(Box::from_raw(self.ptr.as_ptr())) }
        }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.weak.data().data_ref_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            let ptr = self.weak.data().data.get();
            // Safety: The data reference counter is zero,
            // so nothing will access
            unsafe { (*ptr) = None }
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
    fn test_drop_weak() {
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
