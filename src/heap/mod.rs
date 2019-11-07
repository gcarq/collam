use core::ffi::c_void;
use core::{fmt, mem};

use libc_print::libc_eprintln;

use crate::heap::list::IntrusiveList;
use crate::util::align_next_mul_16;

mod list;

static mut HEAP: IntrusiveList = IntrusiveList::new();

pub const BLOCK_REGION_META_SIZE: usize = mem::size_of::<BlockRegion>();
const SPLIT_MIN_BLOCK_SIZE: usize = align_next_mul_16(BLOCK_REGION_META_SIZE * 2);
const BLOCK_PADDING: usize = 0;

#[repr(C)]
pub struct BlockRegion {
    pub size: usize,
    pub magic: u32,
    next: Option<*mut BlockRegion>,
    prev: Option<*mut BlockRegion>,
}

impl BlockRegion {
    #[inline]
    pub const fn new(size: usize) -> Self {
        BlockRegion {
            size,
            next: None,
            prev: None,
            magic: 0xBADC0DED,
        }
    }

    #[inline]
    pub fn verify(&self, panic: bool) -> bool {
        if self.magic != 0xBADC0DED {
            if panic {
                panic!(
                    "[heap] magic value does not match (got=0x{:X}, expected=0xBADC0DED)",
                    self.magic
                );
            }
            error!(
                "[heap] WARN: magic value does not match (got=0x{:X}, expected=0xBADC0DED)",
                self.magic
            );
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
    debug!("[insert]: {} at {:?}", *block, block);
    HEAP.insert(block);
    //if cfg!(debug_assertions) {
    HEAP.debug();
    //}
}

/// Removes and returns a suitable empty block from the heap structure.
#[inline]
pub unsafe fn pop(size: usize) -> Option<*mut BlockRegion> {
    let block = HEAP.pop(size)?;
    debug!("[pop]: {} at {:?}", *block, block);
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
    (*block).verify(true);
    return block.offset(1).cast::<c_void>();
}

/// Splits the given block in-place to have the exact memory size as specified (excluding metadata).
/// Returns a newly created block with the remaining size or None if split is not possible.
pub fn split(block: *mut BlockRegion, size: usize) -> Option<*mut BlockRegion> {
    unsafe { debug!("[split]: {} at {:?}", *block, block) }

    let new_blk_offset = align_next_mul_16(BLOCK_REGION_META_SIZE + size + BLOCK_PADDING);
    // Check if its possible to split the block with the requested size
    let new_blk_size = unsafe { (*block).size }
        .checked_sub(BLOCK_PADDING)?
        .checked_sub(new_blk_offset)?
        .checked_sub(BLOCK_REGION_META_SIZE)?;

    if new_blk_size < SPLIT_MIN_BLOCK_SIZE {
        debug!("      -> None");
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

        debug!("      -> {} at {:?}", *block, block);
        debug!("      -> {} at {:?}", *new_block, new_block);
        debug!(
            "         distance is {} bytes",
            new_block as usize - (block as usize + BLOCK_REGION_META_SIZE + (*block).size)
        );
        assert_eq!(
            new_block as usize - (block as usize + BLOCK_REGION_META_SIZE + (*block).size),
            BLOCK_PADDING
        );
        return Some(new_block);
    };
}

/*
/// Iterates over the heap and merges the first match of two continuous unused blocks.
fn scan_merge() {
    let mut ptr = head();
    while let Some(block) = ptr {
        unsafe {
            if let Some(next) = (*block).next {
                if !(*block).used && !(*next).used {
                    merge(block, next);
                }
            }
            ptr = (*block).next;
        }
    }
}

/// Takes pointers to two continuous blocks and merges them.
/// Returns a pointer to the merged block.
fn merge(block1: *mut BlockMeta, block2: *mut BlockMeta) {
    unsafe {
        log!("[merge]: {} at {:?}", *block1, block1);
        log!("         {} at {:?}", *block2, block2);
        (*block1).size += BLOCK_META_SIZE + (*block2).size;
        (*block1).next = (*block2).next;
        (*block1).used = false;
        log!("      -> {} at {:?}", *block1, block1);
    }
}*/
