use core::{ffi::c_void, ptr};

use libc_print::libc_eprintln;

use crate::heap::{self, BlockRegion};
use crate::util;
use core::ptr::NonNull;

pub fn alloc(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }
    let size = util::align_next_mul_16(size);
    dprintln!("[libdmalloc.so]: alloc(size={})", size);

    // Check if there is already a suitable block allocated
    let block = if let Some(block) = unsafe { heap::pop(size) } {
        block
    // Request new block from kernel
    } else if let Some(block) = request_block(size) {
        block
    } else {
        dprintln!("[libdmalloc.so]: failed for size: {}\n", size);
        return ptr::null_mut();
    };
    split_insert(block, size);

    unsafe {
        dprintln!("[libdmalloc.so]: returning {} at {:?}\n", block.as_ref(), block);
        debug_assert!(block.as_ref().size >= size, "requested={}, got={}", size, block.as_ref());
        return heap::get_mem_region(block);
    }
}

/// Splits the given block in-place to have the exact memory size as specified (excluding metadata).
/// The remaining block (if any) is added to the heap.
pub fn split_insert(block: NonNull<BlockRegion>, size: usize) {
    if let Some(rem_block) = heap::split(block, size) {
        unsafe { heap::insert(rem_block) };
    }
}

/// Requests memory from kernel and returns a pointer to the newly created BlockMeta.
fn request_block(size: usize) -> Option<NonNull<heap::BlockRegion>> {
    let alloc_unit = util::alloc_unit(heap::BLOCK_REGION_META_SIZE + size);
    let block = sbrk(alloc_unit as isize)?.cast::<heap::BlockRegion>();
    unsafe {
        (*block) = heap::BlockRegion::new(alloc_unit);
    }
    NonNull::new(block)
}

pub fn sbrk(size: isize) -> Option<*mut c_void> {
    let ptr = unsafe { libc::sbrk(size) };
    if ptr == -1_isize as *mut c_void {
        None
    } else {
        Some(ptr)
    }
}
