use core::intrinsics;
use libc_print::libc_println;


static mut HEAD: Option<*mut BlockMeta> = None;

#[repr(C)]
pub struct BlockMeta {
    pub size: usize,
    pub next: Option<*mut BlockMeta>,
    pub empty: bool,
}

pub const BLOCK_META_SIZE: usize = intrinsics::size_of::<BlockMeta>();

use libc::c_void;

pub fn alloc_block(size: usize) -> * mut BlockMeta {
    let block = unsafe { libc::sbrk(0) } as *mut BlockMeta;
    let req = unsafe { libc::sbrk((BLOCK_META_SIZE + size) as isize) };
    unsafe {
        (*block).size = size;
        (*block).next = None;
        (*block).empty = false;
    }
    libc_println!("[libdmalloc.so] DEBUG: alloc_block() BlockMeta starts at {:?} (meta_size={})", req, BLOCK_META_SIZE);
    assert!(block as *mut c_void == req);
    unsafe {
        match HEAD {
            None => HEAD = Some(block),
            Some(b) => {(*b).next = Some(block)}
        }
    }
    block
}