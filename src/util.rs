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
/// Rounded up size is:
/// size_rounded_up = (size + align - 1) & !(align - 1);
///
/// We know from above that align != 0. If adding align
/// does not overflow, then rounding up will be fine.
///
/// Conversely, &-masking with !(align - 1) will subtract off
/// only low-order-bits. Thus if overflow occurs with the sum,
/// the &-mask cannot subtract enough to undo that overflow.
///
/// Above implies that checking for summation overflow is both
/// necessary and sufficient.
#[inline]
pub fn align_val(val: usize, align: usize) -> Result<usize, ()> {
    if val > usize::max_value() - align {
        return Err(());
    }
    Ok(align_val_unchecked(val, align))
}

/// Aligns passed value to align and returns it.
/// NOTE: not checked for overflows!
#[inline]
pub const fn align_val_unchecked(val: usize, align: usize) -> usize {
    (val + align - 1) & !(align - 1)
}

/// Aligns passed value to be at lest the size of the
/// largest scalar type `libc::max_align_t` and returns it.
#[inline]
pub fn align_scalar(val: usize) -> Result<usize, ()> {
    align_val(val, align_of::<libc::max_align_t>())
}

/// Aligns passed value to be at lest the size of the
/// largest scalar type `libc::max_align_t` and returns it.
/// NOTE: not checked for overflows!
#[inline]
pub const fn align_scalar_unchecked(val: usize) -> usize {
    align_val_unchecked(val, align_of::<libc::max_align_t>())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_val_ok() {
        let align = 4096;
        for val in [0, 5, 491, 5910, 15290, 501920].iter() {
            assert_eq!(align_val(*val, align).expect("unable to align") % align, 0);
        }
    }

    #[test]
    fn test_align_val_err() {
        assert_eq!(align_val(usize::max_value() - 12, 4096), Err(()));
    }

    #[test]
    fn test_align_val_unchecked() {
        let align = 4096;
        for val in [0, 5, 491, 5910, 15290, 501920].iter() {
            assert_eq!(align_val_unchecked(*val, align) % align, 0);
        }
    }

    #[test]
    fn test_align_scalar_ok() {
        let align = align_of::<libc::max_align_t>();
        for val in [0, 5, 491, 5910, 15290, 501920].iter() {
            assert_eq!(align_scalar(*val).expect("unable to align") % align, 0);
        }
    }

    #[test]
    fn test_align_scalar_err() {
        assert_eq!(align_scalar(usize::max_value() - 15), Err(()));
    }

    #[test]
    fn test_align_scalar_unchecked() {
        let align = align_of::<libc::max_align_t>();
        for val in [0, 5, 491, 5910, 15290, 501920].iter() {
            assert_eq!(align_scalar_unchecked(*val) % align, 0);
        }
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
