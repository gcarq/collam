use core::ffi::c_void;

use libc_print::libc_eprintln;

use crate::heap::list::{BlockRegion, IntrusiveList};

pub mod list;

static mut HEAP: IntrusiveList = IntrusiveList::new();
//static mut HEAD: Option<*mut BlockMeta> = None;

/// Inserts a block to the heap structure
pub fn insert(block: *mut BlockRegion) {
    unsafe {
        log!("[insert]: {} at {:?}", *block, block);
        HEAP.insert(block);
    }
}

pub fn remove(block: *mut BlockRegion) {
    unsafe {
        log!("[remove]: {} at {:?}", *block, block);
        HEAP.remove(block);
    }
}

pub fn split(block: *mut BlockRegion, size: usize) -> Option<*mut BlockRegion> {
    unsafe { HEAP.split(block, size) }
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

pub fn stat() {
    if cfg!(debug_assertions) {
        unsafe { HEAP.debug() }
    }
}

pub fn find_suitable_block(size: usize) -> Option<*mut BlockRegion> {
    unsafe { HEAP.find_block(size) }
}
