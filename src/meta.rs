use core::intrinsics;
use core::ffi::c_void;
use libc_print::libc_eprintln;


pub const BLOCK_META_SIZE: usize = intrinsics::size_of::<BlockMeta>();
static mut HEAD: Option<*mut BlockMeta> = None;

#[repr(C)]
pub struct BlockMeta {
    pub size: usize,
    pub next: Option<*mut BlockMeta>,
    pub empty: bool,
}

pub fn alloc_block(size: usize) -> Option<* mut BlockMeta> {
    let block = sbrk(0)? as *mut BlockMeta;
    let raw_size = (BLOCK_META_SIZE + size).next_power_of_two();
    let requested = sbrk(raw_size as isize)?;
    unsafe {
        (*block).size = size;
        (*block).next = None;
        (*block).empty = false;
    }
    libc_eprintln!("[libdmalloc.so] DEBUG: alloc_block() BlockMeta starts at {:?} (meta_size={}, raw_size={})", requested, BLOCK_META_SIZE, raw_size);
    assert_eq!(block as *mut c_void, requested);
    update_heap(block);
    Some(block)
}

pub fn reuse_block(size: usize) -> Option<*mut BlockMeta> {
    let mut cur_block = unsafe { HEAD };
    while let Some(block) = cur_block {
        unsafe {
            //libc_println!("[libdmalloc.so] DEBUG: reuse_block() checking {:?} (empty={}, size={}, next={:?})", block, (*block).empty, (*block).size, (*block).next);
            if (*block).empty && size <= (*block).size {
                (*block).empty = false;
                return Some(block);
            }
            cur_block = (*block).next;
        }
    }
    None
}

fn update_heap(block: *mut BlockMeta) {
    unsafe {
        match HEAD {
            None => HEAD = Some(block),
            Some(b) => {(*b).next = Some(block)}
        }
    }
}

/// Returns a pointer to the BlockMeta struct from the given memory region raw pointer
pub fn get_block_meta(ptr: *mut c_void) -> *mut BlockMeta {
    unsafe {(ptr as *mut BlockMeta).offset(-1)}
}

/// Returns a pointer to the assigned memory region for the given block
pub fn get_mem_region(block: *mut BlockMeta) -> *mut c_void {
    unsafe { block.offset(1) as *mut c_void }
}

fn sbrk(size: isize) -> Option<*mut c_void> {
    let ptr = unsafe { libc::sbrk(size) };
    if ptr == -1_isize as *mut c_void {
        None
    } else {
        Some(ptr)
    }
}