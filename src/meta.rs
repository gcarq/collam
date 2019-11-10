use core::ffi::c_void;
use core::ptr;

use libc_print::libc_eprintln;

use crate::heap;
use crate::util;

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

    if let Some(rem_block) = heap::split(block, size) {
        unsafe { heap::insert(rem_block) };
    }

    unsafe {
        dprintln!("[libdmalloc.so]: returning {} at {:?}\n", *block, block);
        debug_assert!((*block).size >= size, "requested={}, got={}", size, *block);
        return heap::get_mem_region(block);
    }
}

/// Requests memory from kernel and returns a pointer to the newly created BlockMeta.
fn request_block(size: usize) -> Option<*mut heap::BlockRegion> {
    let alloc_unit = util::alloc_unit(heap::BLOCK_REGION_META_SIZE + size);
    let block = sbrk(alloc_unit as isize)?.cast::<heap::BlockRegion>();
    unsafe {
        (*block) = heap::BlockRegion::new(alloc_unit);
    }
    Some(block)
}

pub fn sbrk(size: isize) -> Option<*mut c_void> {
    let ptr = unsafe { libc::sbrk(size) };
    if ptr == -1_isize as *mut c_void {
        None
    } else {
        Some(ptr)
    }
}
