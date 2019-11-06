use core::ffi::c_void;
use core::{fmt, mem};

use libc_print::libc_eprintln;

use crate::heap::list::IntrusiveList;

mod list;

static mut HEAP: IntrusiveList = IntrusiveList::new();

#[repr(C)]
pub struct BlockRegion {
    pub size: usize,
    next: Option<*mut BlockRegion>,
    prev: Option<*mut BlockRegion>,
}

impl BlockRegion {
    pub fn new(size: usize) -> Self {
        BlockRegion {
            size,
            next: None,
            prev: None,
        }
    }
}

impl fmt::Display for BlockRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BlockRegion(size={}, prev={:?}, next={:?}, meta_size={})",
            self.size, self.prev, self.next, BLOCK_REGION_META_SIZE,
        )
    }
}

pub const BLOCK_REGION_META_SIZE: usize = mem::size_of::<BlockRegion>();

/// Inserts a block to the heap structure
#[inline]
pub fn insert(block: *mut BlockRegion) {
    unsafe {
        log!("[insert]: {} at {:?}", *block, block);
        HEAP.insert(block);
    }
}

/// Removes the given block from the heap
#[inline]
pub fn remove(block: *mut BlockRegion) {
    unsafe {
        log!("[remove]: {} at {:?}", *block, block);
        HEAP.remove(block);
    }
}

/// Returns a suitable empty block from the heap structure
#[inline]
pub fn find(size: usize) -> Option<*mut BlockRegion> {
    unsafe { HEAP.find_block(size) }
}

/// Prints some debugging information about the heap structure
#[inline]
pub fn debug() {
    if cfg!(debug_assertions) {
        unsafe { HEAP.debug() }
    }
}

/// Returns a pointer to the BlockMeta struct from the given memory region raw pointer
#[inline]
pub fn get_block_meta(ptr: *mut c_void) -> *mut BlockRegion {
    unsafe { ptr.cast::<BlockRegion>().offset(-1) }
}

/// Returns a pointer to the assigned memory region for the given block
#[inline]
pub fn get_mem_region(block: *mut BlockRegion) -> *mut c_void {
    unsafe { block.offset(1).cast::<c_void>() }
}

/// Splits the given block in-place to have the exact memory size as specified (excluding metadata).
/// Returns a newly created block with the remaining size or None if split is not possible.
pub fn split(block: *mut BlockRegion, size: usize) -> Option<*mut BlockRegion> {
    unsafe { log!("[split]: {} at {:?}", *block, block) }

    // Align pointer of new block
    let new_blk_offset = (BLOCK_REGION_META_SIZE + size + 1).next_power_of_two();
    // Check if its possible to split the block with the requested size
    let new_blk_size = unsafe { (*block).size }
        .checked_sub(new_blk_offset)?
        .checked_sub(BLOCK_REGION_META_SIZE)?;
    if new_blk_size == 0 {
        log!("      -> None");
        return None;
    }

    unsafe {
        assert!(
            new_blk_offset + BLOCK_REGION_META_SIZE < (*block).size,
            "(left={}, right={})",
            new_blk_offset + BLOCK_REGION_META_SIZE,
            (*block).size
        );

        // Update size for old block
        (*block).size = size;

        // Create block with remaining size
        let new_block = block
            .cast::<c_void>()
            .offset(new_blk_offset as isize)
            .cast::<BlockRegion>();
        *new_block = BlockRegion::new(new_blk_size);

        log!("      -> {} at {:?}", *block, block);
        log!("      -> {} at {:?}", *new_block, new_block);

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
}*/

/*
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
