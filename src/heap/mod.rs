use core::{ffi::c_void, fmt, mem, ptr::Unique};

use libc_print::libc_eprintln;

use crate::heap::list::IntrusiveList;
#[cfg(feature = "stats")]
use crate::stats;
use crate::util;

mod list;

static mut HEAP: IntrusiveList = IntrusiveList::new();

pub const BLOCK_REGION_META_SIZE: usize = mem::size_of::<BlockRegion>();
const SPLIT_MIN_BLOCK_SIZE: usize = util::align_scalar(BLOCK_REGION_META_SIZE * 2);
const BLOCK_MAGIC_FREE: u16 = 0xDEAD;
const BLOCK_MAGIC_USED: u16 = 0xDA7A;

#[repr(C)]
pub struct BlockRegion {
    pub size: usize,
    next: Option<Unique<BlockRegion>>,
    prev: Option<Unique<BlockRegion>>,
    pub magic: u16,
}

impl BlockRegion {
    #[inline]
    pub const fn new(size: usize) -> Self {
        BlockRegion {
            size,
            next: None,
            prev: None,
            magic: BLOCK_MAGIC_FREE,
        }
    }

    #[inline]
    pub fn verify(&self, panic: bool, warn: bool) -> bool {
        if self.magic != BLOCK_MAGIC_FREE {
            if panic {
                panic!(
                    "[heap] magic value does not match (got=0x{:X}, expected=0x{:X})",
                    self.magic, BLOCK_MAGIC_FREE
                );
            }
            if warn {
                eprintln!(
                    "[heap] WARN: magic value does not match (got=0x{:X}, expected=0x{:X})",
                    self.magic, BLOCK_MAGIC_FREE
                );
            }
            return false;
        }
        return true;
    }
}

impl fmt::Display for BlockRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BlockRegion(size={}, prev={:?}, next={:?}, magic=0x{:X}, meta_size={})",
            self.size, self.prev, self.next, self.magic, BLOCK_REGION_META_SIZE,
        )
    }
}

/// Inserts a block to the heap structure.
/// The block is returned to the OS if blocks end is equivalent to program break.
pub unsafe fn insert(block: Unique<BlockRegion>) {
    #[cfg(feature = "debug")]
    HEAP.debug();
    #[cfg(feature = "stats")]
    {
        stats::update_ends(HEAP.head, HEAP.tail);
        stats::print();
    }

    let ptr = get_next_potential_block_ptr(block);
    if let Some(brk) = util::get_program_break() {
        if ptr.as_ptr() == brk.as_ptr() {
            let offset = BLOCK_REGION_META_SIZE + block.as_ref().size;
            dprintln!(
                "[insert]: freeing {} bytes from process (break={:?})",
                offset,
                ptr
            );
            util::sbrk(-1 * offset as isize);
            return;
        }
    }

    dprintln!("[insert]: {} at {:?}", block.as_ref(), block);
    if HEAP.insert(block).is_err() {
        eprintln!("double free detected for ptr {:?}", get_mem_region(block));
    }
}

/// Removes and returns a suitable empty block from the heap structure.
#[inline(always)]
pub unsafe fn pop(size: usize) -> Option<Unique<BlockRegion>> {
    let block = HEAP.pop(size)?;
    dprintln!("[pop]: {} at {:?}", block.as_ref(), block);
    return Some(block);
}

/// Returns a pointer to the BlockMeta struct from the given memory region raw pointer
#[inline(always)]
pub unsafe fn get_block_meta(ptr: Unique<c_void>) -> Unique<BlockRegion> {
    Unique::new_unchecked(ptr.cast::<BlockRegion>().as_ptr().offset(-1))
}

/// Returns a pointer to the assigned memory region for the given block
#[inline(always)]
pub unsafe fn get_mem_region(block: Unique<BlockRegion>) -> Option<Unique<c_void>> {
    block.as_ref().verify(true, true);
    return Unique::new(block.as_ptr().offset(1).cast::<c_void>());
}

/// Returns a pointer where the next BlockRegion would start.
/// TODO: resolve new_unchecked
#[inline(always)]
unsafe fn get_next_potential_block_ptr(block: Unique<BlockRegion>) -> Unique<c_void> {
    let offset = util::align_scalar(BLOCK_REGION_META_SIZE + block.as_ref().size) as isize;
    return Unique::new_unchecked(block.cast::<c_void>().as_ptr().offset(offset));
}

/// Splits the given block in-place to have the exact memory size as specified (excluding metadata).
/// Returns a newly created block with the remaining size or None if split is not possible.
pub fn split(mut block: Unique<BlockRegion>, size: usize) -> Option<Unique<BlockRegion>> {
    unsafe { dprintln!("[split]: {} at {:?}", block.as_ref(), block) }
    debug_assert_eq!(size, util::align_scalar(size));
    let new_blk_offset = util::align_scalar(BLOCK_REGION_META_SIZE + size);
    // Check if its possible to split the block with the requested size
    let new_blk_size = unsafe { block.as_ref().size }
        .checked_sub(new_blk_offset)?
        .checked_sub(BLOCK_REGION_META_SIZE)?;

    if new_blk_size < SPLIT_MIN_BLOCK_SIZE {
        dprintln!("      -> None");
        return None;
    }

    unsafe {
        // Update size for old block
        block.as_mut().size = size;
        // Create block with remaining size
        let new_block = block
            .cast::<c_void>()
            .as_ptr()
            .offset(new_blk_offset as isize)
            .cast::<BlockRegion>();
        *new_block = BlockRegion::new(new_blk_size);

        dprintln!("      -> {} at {:?}", block.as_ref(), block);
        dprintln!("      -> {} at {:?}", *new_block, new_block);
        dprintln!(
            "         distance is {} bytes",
            new_block as usize
                - (block.as_ptr() as usize + BLOCK_REGION_META_SIZE + block.as_ref().size)
        );
        debug_assert_eq!(
            new_block as usize
                - (block.as_ptr() as usize + BLOCK_REGION_META_SIZE + block.as_ref().size),
            0
        );
        return Unique::new(new_block);
    };
}

pub fn alloc(size: usize) -> Option<Unique<c_void>> {
    if size == 0 {
        return None;
    }

    dprintln!("[libdmalloc.so]: alloc(size={})", size);
    let size = util::align_scalar(size);
    // Check if there is already a suitable block allocated
    let block = if let Some(block) = unsafe { pop(size) } {
        block
    // Request new block from kernel
    } else if let Some(block) = request_block(size) {
        block
    } else {
        dprintln!("[libdmalloc.so]: failed for size: {}\n", size);
        return None;
    };
    split_insert(block, size);

    unsafe {
        dprintln!(
            "[libdmalloc.so]: returning {} at {:?}\n",
            block.as_ref(),
            block
        );
        debug_assert!(
            block.as_ref().size >= size,
            "requested={}, got={}",
            size,
            block.as_ref()
        );
        return get_mem_region(block);
    }
}

/// Splits the given block in-place to have the exact memory size as specified (excluding metadata).
/// The remaining block (if any) is added to the heap.
#[inline]
pub fn split_insert(block: Unique<BlockRegion>, size: usize) {
    if let Some(rem_block) = split(block, size) {
        unsafe { insert(rem_block) };
    }
}

/// Requests memory from kernel and returns a pointer to the newly created BlockMeta.
fn request_block(size: usize) -> Option<Unique<BlockRegion>> {
    let alloc_unit = util::alloc_unit(BLOCK_REGION_META_SIZE + size);
    let block = util::sbrk(alloc_unit as isize)?.cast::<BlockRegion>();
    unsafe {
        (*block.as_ptr()) = BlockRegion::new(alloc_unit);
    }
    Some(block)
}
