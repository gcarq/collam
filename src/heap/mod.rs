use core::{ffi::c_void, ptr::Unique};

use libc_print::libc_eprintln;

use crate::heap::list::IntrusiveList;
use crate::heap::region::{BlockRegionPtr, BLOCK_REGION_META_SIZE};
#[cfg(feature = "stats")]
use crate::stats;
use crate::util;

mod list;
pub mod region;

static mut HEAP: IntrusiveList = IntrusiveList::new();

/// Inserts a block to the heap structure.
/// The block is returned to the OS if blocks end is equivalent to program break.
pub unsafe fn insert(mut block: BlockRegionPtr) {
    #[cfg(feature = "debug")]
    HEAP.debug();
    #[cfg(feature = "stats")]
    {
        stats::update_ends(HEAP.head, HEAP.tail);
        stats::print();
    }

    let ptr = block.next_potential_block();
    if let Some(brk) = util::sbrk(0) {
        if ptr.as_ptr() == brk.as_ptr() {
            // TODO: make sure value doesn't overflow
            let offset = block.raw_size() as isize;
            dprintln!(
                "[insert]: freeing {} bytes from process (break={:?})",
                offset,
                ptr
            );
            util::sbrk(-offset);
            return;
        }
    }

    block.as_mut().prev = None;
    block.as_mut().next = None;
    dprintln!("[insert]: {} at {:p}", block.as_ref(), block);
    if HEAP.insert(block).is_err() {
        eprintln!("double free detected for ptr {:?}", block.mem_region());
    }
}

/// Removes and returns a suitable empty block from the heap structure.
#[inline(always)]
pub unsafe fn pop(size: usize) -> Option<BlockRegionPtr> {
    let block = HEAP.pop(size)?;
    dprintln!("[pop]: {} at {:p}", block.as_ref(), block);
    return Some(block);
}

pub fn alloc(size: usize) -> Option<Unique<c_void>> {
    if size == 0 {
        return None;
    }

    dprintln!("[libdmalloc.so]: alloc(size={})", size);
    let size = util::align_scalar(size);
    // Check if there is already a suitable block allocated
    let mut block = if let Some(block) = unsafe { pop(size) } {
        block
    // Request new block from kernel
    } else if let Some(block) = request_block(size) {
        block
    } else {
        dprintln!("[libdmalloc.so]: failed for size: {}\n", size);
        return None;
    };
    split_insert(&mut block, size);

    dprintln!(
        "[libdmalloc.so]: returning {} at {:p}\n",
        block.as_ref(),
        block
    );
    debug_assert!(
        block.size() >= size,
        "requested_size={}, got_block={}",
        size,
        block.as_ref()
    );
    return Some(block.mem_region());
}

/// Splits the given block in-place to have the exact memory size as specified (excluding metadata).
/// The remaining block (if any) is added to the heap.
#[inline]
pub fn split_insert(block: &mut BlockRegionPtr, size: usize) {
    if let Some(rem_block) = block.shrink(size) {
        unsafe { insert(rem_block) };
    }
}

/// Requests memory from kernel and returns a pointer to the newly created BlockMeta.
fn request_block(size: usize) -> Option<BlockRegionPtr> {
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    let alloc_size = util::align_val_unchecked(BLOCK_REGION_META_SIZE + size, page_size);
    let ptr = util::sbrk(alloc_size as isize)?;
    return Some(BlockRegionPtr::new(
        ptr.as_ptr(),
        alloc_size - BLOCK_REGION_META_SIZE,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util;

    #[test]
    fn test_request_block() {
        let region = request_block(256).expect("unable to request block");
        let brk = region.next_potential_block().as_ptr();
        assert_eq!(brk, util::sbrk(0).expect("sbrk(0) failed").as_ptr());
    }

    #[test]
    fn test_request_block_split() {
        let rem_region = request_block(256)
            .expect("unable to request block")
            .shrink(128)
            .expect("unable to split block");
        let brk = rem_region.next_potential_block().as_ptr();
        assert_eq!(brk, util::sbrk(0).expect("sbrk(0) failed").as_ptr());
    }
}
