use core::convert::TryFrom;
use core::ptr::Unique;

use crate::alloc::block::{BlockPtr, BLOCK_META_SIZE};
use crate::util;

use libc_print::libc_eprintln;

lazy_static! {
    static ref PAGE_SIZE: usize =
        usize::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).unwrap();
}

pub trait MemorySource {
    /// Requests memory for the minimum specified size from the memory source
    unsafe fn request(&self, size: usize) -> Option<BlockPtr>;
    /// Releases given `BlockPtr` back to the memory source.
    /// Returns `true` if block has been released, `false` otherwise.
    unsafe fn release(&mut self, block: BlockPtr) -> bool;
}

/// Defines data segment as memory source.
/// Makes use of brk(2).
pub struct DataSegment;

impl DataSegment {
    /// Wrapper for the kernel sbrk call.
    ///
    /// # Safety
    ///
    /// Function is not thread safe.
    #[inline]
    unsafe fn sbrk(size: isize) -> Option<Unique<u8>> {
        let ptr = libc::sbrk(size) as *mut u8;
        if ptr == -1_isize as *mut u8 {
            None
        } else {
            Unique::new(ptr)
        }
    }
}

impl MemorySource for DataSegment {
    /// # Safety
    ///
    /// Function is not thread safe.
    unsafe fn request(&self, size: usize) -> Option<BlockPtr> {
        let size = util::pad_to_align(BLOCK_META_SIZE + size, *PAGE_SIZE)
            .ok()?
            .size();
        debug_assert!(size > BLOCK_META_SIZE);
        let offset = isize::try_from(size).expect("cannot calculate sbrk offset");
        Some(BlockPtr::new(Self::sbrk(offset)?, size - BLOCK_META_SIZE))
    }

    /// # Safety
    ///
    /// Function is not thread safe.
    unsafe fn release(&mut self, block: BlockPtr) -> bool {
        let brk = Self::sbrk(0).expect("sbrk(0) failed!").as_ptr();
        if block.next_potential_block().as_ptr() != brk {
            return false;
        }

        let offset = isize::try_from(block.block_size()).expect("cannot calculate sbrk offset");
        dprintln!(
            "[DataSegment]: freeing {} bytes from process (break={:?})",
            offset,
            Self::sbrk(0).expect("sbrk(0) failed!").as_ptr()
        );
        // TODO: remove expect
        Self::sbrk(-offset).expect("sbrk failed");
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbrk_ok() {
        unsafe { assert!(DataSegment::sbrk(0).is_some()) };
    }

    #[test]
    fn test_sbrk_err() {
        unsafe {
            assert!(DataSegment::sbrk(isize::min_value()).is_none());
        }
    }
}
