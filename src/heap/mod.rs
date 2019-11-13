use core::{ffi::c_void, fmt, mem, ptr::NonNull};

use libc_print::libc_eprintln;

use crate::heap::list::IntrusiveList;
use crate::util;

mod list;

static mut HEAP: IntrusiveList = IntrusiveList::new();

pub const BLOCK_REGION_META_SIZE: usize = mem::size_of::<BlockRegion>();
const SPLIT_MIN_BLOCK_SIZE: usize = util::align_val(BLOCK_REGION_META_SIZE * 2);
const BLOCK_MAGIC: u32 = 0xDEADC0DE;

#[repr(C)]
pub struct BlockRegion {
    pub size: usize,
    next: Option<NonNull<BlockRegion>>,
    prev: Option<NonNull<BlockRegion>>,
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

/// Inserts a block to the heap structure.
/// The block is returned to the OS if blocks end is equivalent to program break.
pub unsafe fn insert(block: NonNull<BlockRegion>) {
    let ptr = get_next_potential_block(block).cast::<c_void>();
    if let Some(brk) = util::get_program_break() {
        if ptr == brk {
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
    HEAP.insert(block);
    if cfg!(feature = "debug") {
        HEAP.debug();
    }
}

/// Removes and returns a suitable empty block from the heap structure.
#[inline]
pub unsafe fn pop(size: usize) -> Option<NonNull<BlockRegion>> {
    let block = HEAP.pop(size)?;
    dprintln!("[pop]: {} at {:?}", block.as_ref(), block);
    return Some(block);
}

/// Returns a pointer to the BlockMeta struct from the given memory region raw pointer
#[inline]
pub unsafe fn get_block_meta(ptr: NonNull<c_void>) -> NonNull<BlockRegion> {
    NonNull::new_unchecked(ptr.cast::<BlockRegion>().as_ptr().offset(-1))
}

/// Returns a pointer to the assigned memory region for the given block
#[inline]
pub unsafe fn get_mem_region(block: NonNull<BlockRegion>) -> Option<NonNull<c_void>> {
    block.as_ref().verify(true, true);
    return NonNull::new(block.as_ptr().offset(1).cast::<c_void>());
}

/// Returns a pointer where the next BlockRegion would start.
unsafe fn get_next_potential_block(block: NonNull<BlockRegion>) -> NonNull<BlockRegion> {
    let offset = util::align_val(BLOCK_REGION_META_SIZE + block.as_ref().size) as isize;
    let ptr = block.cast::<c_void>().as_ptr().offset(offset);
    let rel_block = NonNull::new_unchecked(ptr.cast::<BlockRegion>());
    rel_block.as_ref().verify(false, false);
    return rel_block;
}

/// Splits the given block in-place to have the exact memory size as specified (excluding metadata).
/// Returns a newly created block with the remaining size or None if split is not possible.
pub fn split(mut block: NonNull<BlockRegion>, size: usize) -> Option<NonNull<BlockRegion>> {
    unsafe { dprintln!("[split]: {} at {:?}", block.as_ref(), block) }

    let new_blk_offset = util::align_val(BLOCK_REGION_META_SIZE + size);
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
        return NonNull::new(new_block);
    };
}
