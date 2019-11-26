use core::{ffi::c_void, fmt, mem, ptr::Unique};

use libc_print::libc_eprintln;

use crate::util;

pub const BLOCK_REGION_META_SIZE: usize = mem::size_of::<BlockRegion>();
// Minimum size (without meta) for a new block after splitting
pub const SPLIT_MIN_BLOCK_SIZE: usize = mem::align_of::<libc::max_align_t>();
const BLOCK_MAGIC_FREE: u16 = 0xDEAD;
const BLOCK_MAGIC_USED: u16 = 0xDA7A;

/// Represents a mutable non-null Pointer to a BlockRegion.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct BlockRegionPtr(Unique<BlockRegion>);

impl BlockRegionPtr {
    /// Creates a BlockRegion instance at the given raw pointer for the specified size
    pub unsafe fn new(ptr: *mut c_void, size: usize) -> Self {
        let ptr = ptr.cast::<BlockRegion>();
        *ptr = BlockRegion {
            size,
            next: None,
            prev: None,
            magic: BLOCK_MAGIC_FREE,
        };
        return BlockRegionPtr(Unique::new_unchecked(ptr));
    }

    /// Returns an existing BlockRegionPtr instance from the given memory region raw pointer
    #[inline(always)]
    pub unsafe fn from_mem_region(ptr: Unique<c_void>) -> Self {
        BlockRegionPtr(Unique::new_unchecked(
            ptr.cast::<BlockRegion>().as_ptr().offset(-1),
        ))
    }

    /// Returns a pointer to the assigned memory region for the given block
    #[inline(always)]
    pub fn mem_region(&self) -> Option<Unique<c_void>> {
        self.verify(true, true);
        return unsafe { Unique::new(self.0.as_ptr().offset(1).cast::<c_void>()) };
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
    pub unsafe fn next_potential_block(&self) -> Unique<c_void> {
        let offset = util::align_scalar(BLOCK_REGION_META_SIZE + self.as_ref().size) as isize;
        return Unique::new_unchecked(self.cast::<c_void>().as_ptr().offset(offset));
    }

    /// Splits the given block in-place to have the exact memory size as specified (excluding metadata).
    /// Returns a newly created block with the remaining size or None if split is not possible.
    pub fn split(&mut self, size: usize) -> Option<BlockRegionPtr> {
        dprintln!("[split]: {} at {:p}", self.as_ref(), self);
        debug_assert_eq!(size, util::align_scalar(size));
        let new_blk_offset = util::align_scalar(BLOCK_REGION_META_SIZE + size);
        // Check if its possible to split the block with the requested size
        let new_blk_size = self
            .as_ref()
            .size
            .checked_sub(new_blk_offset)?
            .checked_sub(BLOCK_REGION_META_SIZE)?;

        if new_blk_size < SPLIT_MIN_BLOCK_SIZE {
            dprintln!("      -> None");
            return None;
        }

        unsafe {
            // Update size for old block
            self.as_mut().size = size;
            // Create block with remaining size
            let new_ptr = self
                .cast::<c_void>()
                .as_ptr()
                .offset(new_blk_offset as isize);
            let new_block = BlockRegionPtr::new(new_ptr, new_blk_size);

            dprintln!("      -> {} at {:p}", self.as_ref(), self);
            dprintln!("      -> {} at {:p}", new_block.as_ref(), new_block);
            dprintln!(
                "         distance is {} bytes",
                new_block.as_ptr() as usize
                    - (self.as_ptr() as usize + BLOCK_REGION_META_SIZE + self.as_ref().size)
            );
            debug_assert_eq!(
                new_block.as_ptr() as usize
                    - (self.as_ptr() as usize + BLOCK_REGION_META_SIZE + self.as_ref().size),
                0
            );
            return Some(new_block);
        };
    }

    #[inline]
    pub fn verify(&self, panic: bool, warn: bool) -> bool {
        let magic = self.as_ref().magic;
        if magic != BLOCK_MAGIC_FREE {
            if panic {
                panic!(
                    "[heap] magic value does not match (got=0x{:X}, expected=0x{:X})",
                    magic, BLOCK_MAGIC_FREE
                );
            }
            if warn {
                eprintln!(
                    "[heap] WARN: magic value does not match (got=0x{:X}, expected=0x{:X})",
                    magic, BLOCK_MAGIC_FREE
                );
            }
            return false;
        }
        return true;
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

#[repr(C)]
pub struct BlockRegion {
    pub size: usize,
    pub next: Option<BlockRegionPtr>,
    pub prev: Option<BlockRegionPtr>,
    magic: u16,
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
