use core::ffi::c_void;
use core::{fmt, mem};

use libc_print::libc_eprintln;

use crate::heap::list::IntrusiveList;
use crate::meta;
use crate::util;

mod list;

static mut HEAP: IntrusiveList = IntrusiveList::new();

pub const BLOCK_REGION_META_SIZE: usize = mem::size_of::<BlockRegion>();
const SPLIT_MIN_BLOCK_SIZE: usize = util::align_next_mul_16(BLOCK_REGION_META_SIZE * 2);
const BLOCK_MAGIC: u32 = 0xDEADC0DE;

pub struct BlockRegion {
    pub size: usize,
    next: Option<*mut BlockRegion>,
    prev: Option<*mut BlockRegion>,
    pub magic: u32,
}

impl BlockRegion {
    #[inline]
    pub const fn new(size: usize) -> Self {
        BlockRegion {
            size,
            next: None,
            prev: None,
            magic: BLOCK_MAGIC,
        }
    }

    #[inline]
    pub fn verify(&self, panic: bool, warn: bool) -> bool {
        if self.magic != BLOCK_MAGIC {
            if panic {
                panic!(
                    "[heap] magic value does not match (got=0x{:X}, expected=0x{:X})",
                    self.magic, BLOCK_MAGIC
                );
            }
            if warn {
                eprintln!(
                    "[heap] WARN: magic value does not match (got=0x{:X}, expected=0x{:X})",
                    self.magic, BLOCK_MAGIC
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

/// Inserts a block to the heap structure
#[inline]
pub unsafe fn insert(block: *mut BlockRegion) {
    let ptr = get_next_potential_block(block).cast::<c_void>();
    if ptr == util::get_program_break() {
        let dec = BLOCK_REGION_META_SIZE + (*block).size;
        dprintln!(
            "[insert]: freeing {} bytes from process (break={:?})",
            dec,
            ptr
        );
        // TODO: handle sbrk return value
        meta::sbrk(-1 * dec as isize);
        return;
    }

    dprintln!("[insert]: {} at {:?}", *block, block);
    HEAP.insert(block);
    if cfg!(feature = "debug") {
        HEAP.debug();
    }
}

/// Removes and returns a suitable empty block from the heap structure.
#[inline]
pub unsafe fn pop(size: usize) -> Option<*mut BlockRegion> {
    let block = HEAP.pop(size)?;
    dprintln!("[pop]: {} at {:?}", *block, block);
    return Some(block);
}

/// Returns a pointer to the BlockMeta struct from the given memory region raw pointer
#[inline]
pub unsafe fn get_block_meta(ptr: *mut c_void) -> *mut BlockRegion {
    ptr.cast::<BlockRegion>().offset(-1)
}

/// Returns a pointer to the assigned memory region for the given block
#[inline]
pub unsafe fn get_mem_region(block: *mut BlockRegion) -> *mut c_void {
    (*block).verify(true, true);
    return block.offset(1).cast::<c_void>();
}

/// Returns a pointer where the next BlockRegion would start.
unsafe fn get_next_potential_block(block: *mut BlockRegion) -> *mut BlockRegion {
    let offset = util::align_next_mul_16(BLOCK_REGION_META_SIZE + (*block).size) as isize;
    let rel_block = block.cast::<c_void>().offset(offset).cast::<BlockRegion>();
    (*rel_block).verify(false, false);
    return rel_block;
}

/// Splits the given block in-place to have the exact memory size as specified (excluding metadata).
/// Returns a newly created block with the remaining size or None if split is not possible.
pub fn split(block: *mut BlockRegion, size: usize) -> Option<*mut BlockRegion> {
    unsafe { dprintln!("[split]: {} at {:?}", *block, block) }

    let new_blk_offset = util::align_next_mul_16(BLOCK_REGION_META_SIZE + size);
    // Check if its possible to split the block with the requested size
    let new_blk_size = unsafe { (*block).size }
        .checked_sub(new_blk_offset)?
        .checked_sub(BLOCK_REGION_META_SIZE)?;

    if new_blk_size < SPLIT_MIN_BLOCK_SIZE {
        dprintln!("      -> None");
        return None;
    }

    unsafe {
        // Update size for old block
        (*block).size = size;
        // Create block with remaining size
        let new_block = block
            .cast::<c_void>()
            .offset(new_blk_offset as isize)
            .cast::<BlockRegion>();
        *new_block = BlockRegion::new(new_blk_size);

        dprintln!("      -> {} at {:?}", *block, block);
        dprintln!("      -> {} at {:?}", *new_block, new_block);
        dprintln!(
            "         distance is {} bytes",
            new_block as usize - (block as usize + BLOCK_REGION_META_SIZE + (*block).size)
        );
        debug_assert_eq!(
            new_block as usize - (block as usize + BLOCK_REGION_META_SIZE + (*block).size),
            0
        );
        return Some(new_block);
    };
}
