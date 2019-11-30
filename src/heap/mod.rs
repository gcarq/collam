use core::{ffi::c_void, ptr::Unique};

use libc_print::libc_eprintln;

use crate::heap::block::{BlockPtr, BLOCK_META_SIZE};
use crate::heap::list::IntrusiveList;
#[cfg(feature = "stats")]
use crate::stats;
use crate::util;

pub mod block;
mod list;

static mut HEAP: IntrusiveList = IntrusiveList::new();

/// Inserts a `BlockPtr` to the heap structure.
/// NOTE: The memory is returned to the OS if it is adjacent to program break.
pub unsafe fn insert(mut block: BlockPtr) {
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
            let offset = block.block_size() as isize;
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

/// Removes and returns a suitable empty `BlockPtr` from the heap structure.
#[inline(always)]
pub unsafe fn pop(size: usize) -> Option<BlockPtr> {
    let block = HEAP.pop(size)?;
    dprintln!("[pop]: {} at {:p}", block.as_ref(), block);
    return Some(block);
}

/// Find a usable memory region for the given size either by
/// reusing or requesting memory from the kernel.
/// Returns a `Unique<c_void>` pointer to the memory region.
pub fn alloc(size: usize) -> Option<Unique<c_void>> {
    if size == 0 {
        return None;
    }

    dprintln!("[libdmalloc.so]: alloc(size={})", size);
    let size = util::align_scalar(size).ok()?;
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
    shrink_insert_rem(&mut block, size);

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

/// Shrinks the given `BlockPtr` in-place to have
/// the exact memory size as specified (excluding metadata).
/// and adds the remaining block to heap if any.
#[inline]
pub fn shrink_insert_rem(block: &mut BlockPtr, size: usize) {
    if let Some(rem_block) = block.shrink(size) {
        unsafe { insert(rem_block) };
    }
}

/// Requests memory for the specified size from kernel
/// and returns a `BlockPtr` to the newly created block or `None` if not possible.
fn request_block(min_size: usize) -> Option<BlockPtr> {
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    let size = util::align_val(BLOCK_META_SIZE + min_size, page_size).ok()?;
    let ptr = util::sbrk(size as isize)?;
    Some(BlockPtr::new(ptr.as_ptr(), size - BLOCK_META_SIZE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util;

    #[test]
    fn test_request_block() {
        let block = request_block(256).expect("unable to request block");
        let brk = block.next_potential_block().as_ptr();
        assert_eq!(brk, util::sbrk(0).expect("sbrk(0) failed").as_ptr());
    }

    #[test]
    fn test_request_block_split() {
        let rem_block = request_block(256)
            .expect("unable to request block")
            .shrink(128)
            .expect("unable to split block");
        let brk = rem_block.next_potential_block().as_ptr();
        assert_eq!(brk, util::sbrk(0).expect("sbrk(0) failed").as_ptr());
    }
}
