use core::ffi::c_void;
use core::mem::align_of;
use core::ptr::Unique;

#[cfg(feature = "stats")]
use crate::stats;

/// Returns a pointer to the current program break
#[inline(always)]
pub fn get_program_break() -> Option<Unique<c_void>> {
    sbrk(0)
}

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

/// Aligns passed value to libc::max_align_t
#[inline(always)]
pub const fn align_val(val: usize, align: usize) -> usize {
    (val + align - 1) & !(align - 1)
}

/// Aligns val to be at lest the size of the largest scalar type (libc::max_align_t)
#[inline(always)]
pub const fn align_scalar(val: usize) -> usize {
    align_val(val, align_of::<libc::max_align_t>())
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
            assert_eq!(align_val(rng.gen(), align) % align, 0);
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
}