use core::sync::atomic::AtomicBool;
use core::sync::atomic::fence;
use core::sync::atomic::Ordering;

use libc_print::libc_eprintln;

// A mutual exclusion primitive based on spinlock.
pub struct Mutex {
    flag: AtomicBool,
}

impl Mutex {
    pub const fn new() -> Mutex {
        Mutex {
            flag: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) {
        //libc_eprintln!("[libdmalloc.so] DEBUG: mutex_lock()");
        while !self.flag.compare_and_swap(false, true, Ordering::Relaxed) {}
        // This fence synchronizes-with store in `unlock`.
        fence(Ordering::Acquire);
    }

    pub fn unlock(&self) {
        //libc_eprintln!("[libdmalloc.so] DEBUG: mutex_unlock()");
        self.flag.store(false, Ordering::Release);
    }
}