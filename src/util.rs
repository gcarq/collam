use crate::MIN_ALIGN;
use core::alloc::{Layout, LayoutErr};

/// Aligns passed value to be at lest the size of `MIN_ALIGN` and returns it.
/// NOTE: not checked for overflows!
#[inline]
pub const fn min_align_unchecked(val: usize) -> usize {
    (val + MIN_ALIGN - 1) & !(MIN_ALIGN - 1)
}

/// Returns a `Layout` padded to `MIN_ALIGN`.
#[inline]
pub fn pad_min_align(size: usize) -> Result<Layout, LayoutErr> {
    pad_to_align(size, MIN_ALIGN)
}

/// Returns a `Layout` padded to align.
#[inline]
pub fn pad_to_align(size: usize, align: usize) -> Result<Layout, LayoutErr> {
    Ok(Layout::from_size_align(size, align)?.pad_to_align())
}

/// Returns the current process id
/// TODO: find a more portable solution
pub fn getpid() -> u64 {
    unsafe { libc::getpid() as u64 }
}

/// Returns the current thread id
/// TODO: find a more portable solution
pub fn gettid() -> u64 {
    unsafe { libc::syscall(libc::SYS_gettid) as u64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_align_unchecked() {
        for val in [0, 5, 491, 5910, 15290, 501920].iter() {
            assert_eq!(min_align_unchecked(*val) % MIN_ALIGN, 0);
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
    fn test_pad_min_align_ok() {
        for val in [0, 5, 491, 5910, 15290, 501920].iter() {
            let layout = pad_min_align(*val).expect("unable to align");
            assert_eq!(layout.size() % MIN_ALIGN, 0);
        }
    }

    #[test]
    fn test_pad_min_align_err() {
        assert!(pad_min_align(usize::max_value() - 14).is_err());
    }
}
