use core::alloc::{Layout, LayoutErr};
use core::ffi::c_void;
use core::intrinsics::unlikely;
use core::mem::align_of;
use core::ptr::Unique;

#[cfg(feature = "stats")]
use crate::stats;

#[inline]
pub fn sbrk(size: isize) -> Option<Unique<c_void>> {
    let ptr = unsafe { libc::sbrk(size) };
    if unsafe { unlikely(ptr == -1_isize as *mut c_void) } {
        return None;
    }
    #[cfg(feature = "stats")]
    unsafe {
        stats::update_heap_info(ptr);
    }
    Unique::new(ptr)
}

/// Aligns passed value to be at lest the size of the
/// largest scalar type `libc::max_align_t` and returns it.
/// NOTE: not checked for overflows!
pub const fn align_scalar_unchecked(val: usize) -> usize {
    let align = align_of::<libc::max_align_t>();
    (val + align - 1) & !(align - 1)
}

/// Returns a `Layout` padded to the largest
/// possible scalar for the current architecture.
#[inline]
pub fn pad_to_scalar(size: usize) -> Result<Layout, LayoutErr> {
    Layout::from_size_align(size, align_of::<libc::max_align_t>())?.pad_to_align()
}

/// Returns a `Layout` padded to align.
#[inline]
pub fn pad_to_align(size: usize, align: usize) -> Result<Layout, LayoutErr> {
    Layout::from_size_align(size, align)?.pad_to_align()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_scalar_unchecked() {
        let align = align_of::<libc::max_align_t>();
        for val in [0, 5, 491, 5910, 15290, 501920].iter() {
            assert_eq!(align_scalar_unchecked(*val) % align, 0);
        }
    }

    #[test]
    fn test_pad_to_align_ok() {
        let align = 4096;
        for val in [0, 5, 491, 5910, 15290, 501920].iter() {
            let layout = pad_to_align(*val, align).expect("unable to align");
            assert_eq!(layout.size() % align, 0);
        }
    }

    #[test]
    fn test_pad_to_align_err() {
        assert!(pad_to_align(usize::max_value() - 12, 4096).is_err());
    }

    #[test]
    fn test_pad_to_scalar_ok() {
        let align = align_of::<libc::max_align_t>();
        for val in [0, 5, 491, 5910, 15290, 501920].iter() {
            let layout = pad_to_scalar(*val).expect("unable to align");
            assert_eq!(layout.size() % align, 0);
        }
    }

    #[test]
    fn test_pad_to_scalar_err() {
        assert!(pad_to_scalar(usize::max_value() - 14).is_err());
    }

    #[test]
    fn test_sbrk_ok() {
        assert!(sbrk(0).is_some())
    }

    #[test]
    fn test_sbrk_err() {
        assert!(sbrk(isize::min_value()).is_none());
    }
}
