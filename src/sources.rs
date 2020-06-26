use core::convert::TryFrom;
use core::ptr::{null_mut, Unique};

use crate::alloc::block::{BlockPtr, BLOCK_META_SIZE};
use crate::util;

use libc_print::libc_eprintln;

pub const INVALID_PTR: *mut u8 = -1_isize as *mut u8;

lazy_static! {
    static ref PAGE_SIZE: usize =
        usize::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).unwrap();
}

pub trait MemorySource {
    /// Creates a new memory source with the given initial size.
    unsafe fn new(size: usize) -> Self;
    /// Requests memory for the minimum specified size from the memory source.
    unsafe fn request(&mut self, size: usize) -> Option<BlockPtr>;
    /// Releases given `BlockPtr` back to the memory source.
    /// Returns `true` if block has been released, `false` otherwise.
    unsafe fn release(&mut self, block: BlockPtr) -> bool;
    /// Returns pointer to allocated memory
    fn ptr(&self) -> Unique<u8>;
    /// Returns total size of allocated memory
    fn size(&self) -> usize;
}

/// Defines heap segment as memory source.
#[repr(C)]
pub struct HeapSegment {
    ptr: Unique<u8>,
    size: usize,
}

impl HeapSegment {
    /// Wrapper for sbrk().
    ///
    /// # Safety
    ///
    /// Function is not thread safe.
    unsafe fn sbrk(size: isize) -> Option<Unique<u8>> {
        match libc::sbrk(size) as *mut u8 {
            INVALID_PTR => None,
            ptr => Unique::new(ptr),
        }
    }
}

impl MemorySource for HeapSegment {
    /// Creates a new memory source with the given initial size.
    ///
    /// # Safety
    ///
    /// Function is not thread safe.
    unsafe fn new(size: usize) -> Self {
        let offset = isize::try_from(size).expect("cannot calculate sbrk offset");
        Self {
            ptr: Self::sbrk(offset).expect("sbrk failed"),
            size,
        }
    }

    /// Requests memory for the minimum specified size from the memory source.
    ///
    /// # Safety
    ///
    /// Function is not thread safe.
    unsafe fn request(&mut self, size: usize) -> Option<BlockPtr> {
        let size = util::pad_to_align(BLOCK_META_SIZE + size, *PAGE_SIZE)
            .ok()?
            .size();
        debug_assert!(size > BLOCK_META_SIZE);
        self.size += size;
        let offset = isize::try_from(size).expect("cannot calculate sbrk offset");
        Some(BlockPtr::new(Self::sbrk(offset)?, size - BLOCK_META_SIZE))
    }

    /// Releases given `BlockPtr` back to the memory source.
    /// Returns `true` if block has been released, `false` otherwise.
    ///
    /// # Safety
    ///
    /// Function is not thread safe.
    unsafe fn release(&mut self, block: BlockPtr) -> bool {
        let brk = Self::sbrk(0).expect("sbrk(0) failed!").as_ptr();
        if block.next_potential_block().as_ptr() != brk {
            return false;
        }

        self.size -= block.block_size();
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

    #[inline]
    fn ptr(&self) -> Unique<u8> {
        self.ptr
    }

    #[inline]
    fn size(&self) -> usize {
        self.size
    }
}

/// Defines mapped memory as memory source.
#[repr(C)]
pub struct MappedMemory {
    ptr: Unique<u8>,
    size: usize,
}

impl MappedMemory {
    /// Wrapper for mmap().
    ///
    /// # Safety
    ///
    /// Function is not thread safe.
    unsafe fn mmap(size: usize) -> Option<Unique<u8>> {
        let prot = libc::PROT_READ | libc::PROT_WRITE;
        let flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS;
        match libc::mmap(null_mut(), size, prot, flags, -1, 0) as *mut u8 {
            INVALID_PTR => None,
            ptr => Unique::new(ptr),
        }
    }
}

impl MemorySource for MappedMemory {
    unsafe fn new(size: usize) -> Self {
        Self {
            ptr: Self::mmap(size).expect("mmap failed"),
            size,
        }
    }

    unsafe fn request(&mut self, _size: usize) -> Option<BlockPtr> {
        None
    }

    unsafe fn release(&mut self, _block: BlockPtr) -> bool {
        false
    }

    #[inline]
    fn ptr(&self) -> Unique<u8> {
        self.ptr
    }

    #[inline]
    fn size(&self) -> usize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbrk_ok() {
        unsafe { assert!(HeapSegment::sbrk(0).is_some()) };
    }

    #[test]
    fn test_sbrk_err() {
        unsafe {
            assert!(HeapSegment::sbrk(isize::min_value()).is_none());
        }
    }
}
