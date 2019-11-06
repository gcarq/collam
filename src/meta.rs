use core::ffi::c_void;
use core::ptr;

use libc_print::libc_eprintln;

use crate::heap::{get_mem_region, BlockRegion, BLOCK_REGION_META_SIZE};
use crate::util::alloc_unit;
use crate::{heap, MUTEX};

pub fn alloc(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }

    let _lock = MUTEX.lock(); // lock gets dropped implicitly
    log!("[libdmalloc.so]: alloc(size={})", size);
    let size = size.next_power_of_two();
    // Check if there is already a suitable block allocated
    let block = if let Some(block) = heap::pop(size) {
        block
    // Request new block from kernel
    } else if let Some(block) = request_block(size) {
        block
    } else {
        return ptr::null_mut();
    };

    if let Some(rem_block) = heap::split(block, size) {
        heap::insert(rem_block);
    }

    heap::debug();
    unsafe {
        log!("[libdmalloc.so]: returning {} at {:?}\n", *block, block);
        assert!((*block).size >= size, "requested={}, got={}", size, *block);
    }
    return unsafe { get_mem_region(block) };
}

/// Requests memory from kernel and returns a pointer to the newly created BlockMeta.
fn request_block(size: usize) -> Option<*mut BlockRegion> {
    let alloc_unit = alloc_unit(BLOCK_REGION_META_SIZE + size);
    let block = sbrk(alloc_unit)?.cast::<BlockRegion>();
    unsafe {
        (*block) = BlockRegion::new(alloc_unit as usize);
    }
    Some(block)
}

fn sbrk(size: isize) -> Option<*mut c_void> {
    let ptr = unsafe { libc::sbrk(size) };
    if ptr == -1_isize as *mut c_void {
        None
    } else {
        Some(ptr)
    }
}
