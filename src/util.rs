use core::ffi::c_void;
use core::mem::align_of;
use core::ptr::Unique;

#[cfg(feature = "stats")]
use crate::stats;

#[inline]
pub fn sbrk(size: isize) -> Option<Unique<c_void>> {
    let ptr = unsafe { libc::sbrk(size) };
    if ptr == -1_isize as *mut c_void {
        return None;
    }
    #[cfg(feature = "stats")]
    unsafe {
        stats::update_heap_info(ptr);
    }
    Unique::new(ptr)
}

/// Aligns passed value to align and returns it.
#[inline(always)]
pub const fn align_val_unchecked(val: usize, align: usize) -> usize {
    /*
    FIXME: can overflow if size is slightly less than usize::MAX
    if size > usize::MAX - (align - 1) {
            return Err(LayoutErr { private: () });
    }
    */
    (val + align - 1) & !(align - 1)
}

/// Aligns passed value to be at lest the size of the
/// largest scalar type `libc::max_align_t` and returns it.
#[inline(always)]
pub const fn align_scalar(val: usize) -> usize {
    align_val_unchecked(val, align_of::<libc::max_align_t>())
}


#[cfg(test)]
mod tests {
    use rand::Rng;
    use super::*;

    #[test]
    fn test_align_val() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let align = 4096;
            assert_eq!(align_val_unchecked(rng.gen(), align) % align, 0);
        }
    }

    #[test]
    fn test_align_scalar() {
        let mut rng = rand::thread_rng();
        let align = align_of::<libc::max_align_t>();
        for _ in 0..100 {
            assert_eq!(align_scalar(rng.gen()) % align, 0);
        }
    }

    #[test]
    fn test_sbrk() {
        assert!(sbrk(0).is_some())
    }
}
