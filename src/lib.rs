#![feature(lang_items, core_intrinsics)]
#![no_std]

extern crate libc;
extern crate libc_print;

use core::panic::PanicInfo;

use libc::{size_t, c_void};
use libc_print::libc_eprintln;
use core::{cmp, intrinsics, ptr};

use crate::meta::{alloc_block, get_block_meta, reuse_block, get_mem_region};

mod macros;
mod meta;
mod util;


#[no_mangle]
pub extern "C" fn malloc(size: size_t) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut::<c_void>();
    }

    // Reuse a free block if applicable
    if let Some(block) = reuse_block(size) {
        let pointer = get_mem_region(block);
        libc_eprintln!("[libdmalloc.so] malloc: reusing block at {:?}", pointer);
        return pointer;
    }

    // Allocate new block with required size
    if let Some(block) = alloc_block(size) {
        let pointer = get_mem_region(block);
        libc_eprintln!("[libdmalloc.so] malloc: allocated {} bytes at {:?}", size, pointer);
        return pointer;
    }

    libc_eprintln!("[libdmalloc.so] malloc failed. retuning NULL!");
    return ptr::null_mut::<c_void>();
}

#[no_mangle]
pub extern "C" fn calloc(nobj: size_t, size: size_t) -> *mut c_void {
    // TODO: check for int overflow
    let total_size = nobj * size;
    let pointer = malloc(total_size);
    unsafe { ptr::write_bytes(pointer, 0, total_size); }
    pointer
}

#[no_mangle]
pub extern "C" fn realloc(p: *mut c_void, size: size_t) -> *mut c_void {
    if p.is_null() {
        return malloc(size);
    }

    let block = get_block_meta(p);
    let new_ptr = malloc(size);
    unsafe {
        // TODO: don't reuse blocks and use copy_nonoverlapping
        ptr::copy(p, new_ptr, cmp::min(size, (*block).size));
    }
    free(p);
    libc_eprintln!("[libdmalloc.so] realloc: reallocated {} bytes at {:?}\n", size, p);
    new_ptr
}

#[no_mangle]
pub extern "C" fn free(pointer: *mut c_void) {
    libc_eprintln!("[libdmalloc.so] free: dropping {:?}\n", pointer);
    if pointer.is_null() {
        return
    }
    let block = get_block_meta(pointer);
    unsafe {
        assert_eq!((*block).empty, false);
        (*block).empty = true;
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        libc_eprintln!("panic occurred: {:?}", s);
    } else {
        libc_eprintln!("panic occurred");
    }
    unsafe { intrinsics::abort() }
}

//#[lang = "panic_fmt"] extern fn panic_fmt() -> ! { unsafe { intrinsics::abort() } }
#[lang = "eh_personality"] extern fn eh_personality() {}
#[lang = "eh_unwind_resume"] extern fn eh_unwind_resume() {}

/*
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
*/