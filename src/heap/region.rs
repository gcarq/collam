use core::{ffi::c_void, fmt, mem, ptr::Unique};

use libc_print::libc_eprintln;

use crate::util;

// The required size to store the bare minimum meta data (size + magic value).
pub const BLOCK_REGION_META_SIZE: usize = util::align_scalar(mem::align_of::<usize>() * 2);
// The minimal size of a block region if not allocated by the user.
// TODO: write better docstring
pub const BLOCK_REGION_MIN_SIZE: usize = util::align_scalar(
    BLOCK_REGION_META_SIZE
        + 2 * mem::align_of::<Option<BlockRegionPtr>>()
        + mem::align_of::<libc::max_align_t>(),
);

const BLOCK_MAGIC_FREE: u16 = 0xDEAD;

/// Represents a mutable non-null Pointer to a BlockRegion.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct BlockRegionPtr(Unique<BlockRegion>);

impl BlockRegionPtr {
    /// Creates a BlockRegion instance at the given raw pointer for the specified size
    pub fn new(ptr: *mut c_void, size: usize) -> Self {
        debug_assert_eq!(size, util::align_scalar(size));
        unsafe {
            let ptr = ptr.cast::<BlockRegion>();
            *ptr = BlockRegion {
                size,
                next: None,
                prev: None,
                magic: BLOCK_MAGIC_FREE,
            };
            return BlockRegionPtr(Unique::new_unchecked(ptr));
        }
    }

    /// Returns an existing BlockRegionPtr instance from the given memory region raw pointer
    #[inline(always)]
    pub unsafe fn from_mem_region(ptr: Unique<c_void>) -> Self {
        let offset = BLOCK_REGION_META_SIZE as isize;
        BlockRegionPtr(Unique::new_unchecked(
            ptr.as_ptr().offset(-offset).cast::<BlockRegion>(),
        ))
    }

    /// Returns a pointer to the assigned memory region for the given block
    #[inline(always)]
    pub fn mem_region(&self) -> Unique<c_void> {
        debug_assert!(self.verify(false));
        return unsafe {
            Unique::new_unchecked(
                self.as_ptr()
                    .cast::<c_void>()
                    .offset(BLOCK_REGION_META_SIZE as isize),
            )
        };
    }

    /// Acquires the underlying `*mut` pointer.
    #[inline(always)]
    pub const fn as_ptr(self) -> *mut BlockRegion {
        self.0.as_ptr()
    }

    /// Casts to a pointer of another type.
    #[inline(always)]
    pub const fn cast<U>(self) -> Unique<U> {
        unsafe { Unique::new_unchecked(self.as_ptr() as *mut U) }
    }

    /// Returns a pointer where the next BlockRegion would start.
    /// TODO: resolve new_unchecked
    #[inline]
    pub fn next_potential_block(&self) -> Unique<c_void> {
        let offset = self.raw_size() as isize;
        return unsafe { Unique::new_unchecked(self.cast::<c_void>().as_ptr().offset(offset)) };
    }

    /// Returns the allocatable size available for the user
    #[inline(always)]
    pub fn size(&self) -> usize {
        return self.as_ref().size;
    }

    /// Returns the raw size in bytes for this memory region
    #[inline(always)]
    pub fn raw_size(&self) -> usize {
        return BLOCK_REGION_META_SIZE + self.size();
    }

    /// Splits the given block in-place to have the exact memory size as specified (excluding metadata).
    /// Returns a newly created block with the remaining size or None if split is not possible.
    pub fn shrink(&mut self, size: usize) -> Option<BlockRegionPtr> {
        dprintln!("[split]: {} at {:p}", self.as_ref(), self);
        debug_assert_eq!(size, util::align_scalar(size));
        // Check if its possible to split the block with the requested size
        let rem_block_size = self.size().checked_sub(size + BLOCK_REGION_META_SIZE)?;

        if rem_block_size < BLOCK_REGION_MIN_SIZE {
            dprintln!("      -> None");
            return None;
        }

        // Update size for old block
        self.as_mut().size = size;

        // Create block with remaining size
        let new_block_ptr = unsafe { self.mem_region().as_ptr().offset(size as isize) };
        let new_block = BlockRegionPtr::new(new_block_ptr, rem_block_size);

        dprintln!("      -> {} at {:p}", self.as_ref(), self);
        dprintln!("      -> {} at {:p}", new_block.as_ref(), new_block);
        dprintln!(
            "         distance is {} bytes",
            new_block.as_ptr() as usize - (self.as_ptr() as usize + self.raw_size())
        );
        debug_assert_eq!(
            new_block.as_ptr() as usize - (self.as_ptr() as usize + self.raw_size()),
            0
        );
        return Some(new_block);
    }

    #[inline]
    pub fn verify(&self, panic: bool) -> bool {
        if self.as_ref().magic == BLOCK_MAGIC_FREE {
            return true;
        }

        if panic {
            panic!(
                "[heap] magic value does not match (got=0x{:X}, expected=0x{:X})",
                self.as_ref().magic,
                BLOCK_MAGIC_FREE
            );
        }
        return false;
    }
}

impl AsMut<BlockRegion> for BlockRegionPtr {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut BlockRegion {
        unsafe { self.0.as_mut() }
    }
}

impl AsRef<BlockRegion> for BlockRegionPtr {
    #[inline(always)]
    fn as_ref(&self) -> &BlockRegion {
        unsafe { self.0.as_ref() }
    }
}

impl PartialEq for BlockRegionPtr {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl fmt::Pointer for BlockRegionPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:p}", self.as_ref())
    }
}

impl fmt::Debug for BlockRegionPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {:p}", self.as_ref(), self)
    }
}

#[repr(C)]
pub struct BlockRegion {
    pub size: usize, // TODO: make private
    magic: u16,
    pub next: Option<BlockRegionPtr>,
    pub prev: Option<BlockRegionPtr>,
}

impl fmt::Display for BlockRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        /*
        TODO: fix formatter for self.prev and self.next
        write!(
            f,
            "BlockRegion(size={}, prev={:?}, next={:?}, magic=0x{:X}, meta_size={})",
            self.size, self.prev, self.next, self.magic, BLOCK_REGION_META_SIZE,
        )*/
        write!(
            f,
            "BlockRegion(size={}, magic=0x{:X}, meta_size={})",
            self.size, self.magic, BLOCK_REGION_META_SIZE,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heap::alloc;

    fn assert_block(block: BlockRegionPtr, size: usize) {
        assert_eq!(block.size(), size, "block size doesn't match");
        assert_eq!(
            block.raw_size(),
            BLOCK_REGION_META_SIZE + size,
            "block raw size doesn't match"
        );
        assert!(block.verify(false), "unable to verify block metadata");
        assert!(block.as_ref().next.is_none(), "next is not None");
        assert!(block.as_ref().prev.is_none(), "prev is not None");
    }

    #[test]
    fn test_block_region_new() {
        let alloc_size = 64;
        let ptr = unsafe { libc::malloc(BLOCK_REGION_META_SIZE + alloc_size) };
        assert_block(BlockRegionPtr::new(ptr, alloc_size), alloc_size);
        unsafe { libc::free(ptr) };
    }

    #[test]
    fn test_block_region_shrink_with_remaining() {
        let block1_size = 4096;
        let ptr = unsafe { libc::malloc(BLOCK_REGION_META_SIZE + block1_size) };
        let mut block1 = BlockRegionPtr::new(ptr, block1_size);
        assert_block(block1, block1_size);
        let total_size = block1.raw_size();
        assert_eq!(ptr, block1.as_ptr().cast::<c_void>());

        // Shrink block1 to 256 bytes
        let mut block2 = block1.shrink(256).expect("split block failed");
        assert_block(block1, 256);
        assert_eq!(
            block1.next_potential_block().as_ptr(),
            block2.cast::<c_void>().as_ptr()
        );
        assert_block(
            block2,
            total_size - block1.raw_size() - BLOCK_REGION_META_SIZE,
        );

        // Shrink block2 to 256 bytes
        let block3 = block2.shrink(256).expect("split block failed");
        assert_block(block2, 256);
        assert_eq!(
            block2.next_potential_block().as_ptr(),
            block3.cast::<c_void>().as_ptr()
        );
        assert_block(
            block3,
            total_size - block1.raw_size() - block2.raw_size() - BLOCK_REGION_META_SIZE,
        );
        unsafe { libc::free(ptr) };
    }

    #[test]
    fn test_block_region_shrink_no_remaining() {
        let alloc_size = 256;
        let ptr = unsafe { libc::malloc(BLOCK_REGION_META_SIZE + alloc_size) };
        let mut block = BlockRegionPtr::new(ptr, alloc_size);
        let remaining = block.shrink(240);

        // Assert correctness of initial block
        assert_eq!(ptr, block.as_ptr().cast::<c_void>());
        assert_block(block, 256);

        // There should be no remaining block
        // since 240 will be aligned to 256 and no space is left.
        assert!(remaining.is_none());
        unsafe { libc::free(ptr) };
    }

    #[test]
    fn test_block_region_verify_ok() {
        let alloc_size = 256;
        let ptr = unsafe { libc::malloc(BLOCK_REGION_META_SIZE + alloc_size) };
        let block = BlockRegionPtr::new(ptr, alloc_size);
        assert_eq!(block.verify(false), true);
        unsafe { libc::free(ptr) };
    }

    #[test]
    fn test_block_region_verify_invalid() {
        let alloc_size = 256;
        let ptr = unsafe { libc::malloc(BLOCK_REGION_META_SIZE + alloc_size) };
        let mut block = BlockRegionPtr::new(ptr, alloc_size);
        block.as_mut().magic = 0x1234;
        assert_eq!(block.verify(false), false);
        unsafe { libc::free(ptr) };
    }

    #[test]
    fn test_block_region_mem_region() {
        let alloc_size = 64;
        let ptr = unsafe { libc::malloc(BLOCK_REGION_META_SIZE + alloc_size) };
        let block = BlockRegionPtr::new(ptr, alloc_size);
        let mem = block.mem_region();
        assert!(mem.as_ptr() > block.as_ptr().cast::<c_void>());
        let block2 = unsafe { BlockRegionPtr::from_mem_region(mem) };
        assert_eq!(block, block2);
        unsafe { libc::free(ptr) };
    }
}
