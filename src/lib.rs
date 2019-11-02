#![feature(lang_items, core_intrinsics, core_panic_info)]
#![no_std]

extern crate libc;
extern crate libc_print;
extern crate spin;

use core::ffi::c_void;
use core::{cmp, intrinsics, panic, ptr};

use libc_print::libc_eprintln;

use crate::meta::{alloc_block, get_block_meta, reuse_block, get_mem_region};

mod macros;
mod meta;
mod util;

static MUTEX: spin::Mutex<()> = spin::Mutex::new(());

#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    return alloc(size);
}

#[no_mangle]
pub extern "C" fn calloc(nobj: usize, size: usize) -> *mut c_void {
    //libc_eprintln!("[libdmalloc.so] calloc (nobj={}, size={})", nobj, size);;
    // TODO: check for int overflow
    let total_size = nobj * size;
    let pointer = alloc(total_size);

    let _lock = MUTEX.lock(); // lock gets dropped implicitly
    unsafe {
        pointer.write_bytes(0, total_size);
    }
    pointer
}

#[no_mangle]
pub extern "C" fn realloc(p: *mut c_void, size: usize) -> *mut c_void {
    let new_ptr = alloc(size);
    // If passed pointer is NULL, just return newly allocated ptr
    if p.is_null() {
        return new_ptr;
    }

    let lock = MUTEX.lock();
    let old_block = get_block_meta(p);
    unsafe {
        // TODO: don't reuse blocks and use copy_nonoverlapping
        new_ptr.copy_from(p, cmp::min(size, (*old_block).size));
    }
    drop(lock);
    free(p);
    //libc_eprintln!("[libdmalloc.so] realloc: reallocated {} bytes at {:?}\n", size, p);
    new_ptr
}

#[no_mangle]
pub extern "C" fn free(pointer: *mut c_void) {
    //FIXME: free gets called with unknown pointer
    if pointer.is_null() {
        return
    }

    let _lock = MUTEX.lock(); // lock gets dropped implicitly
    let block = get_block_meta(pointer);
    //libc_eprintln!("[libdmalloc.so] free: dropping block at {:?}\n", block);
    unsafe {
        assert_eq!((*block).unused, false, "{} at {:?}", *block, block);
        (*block).unused = true;
    }
}

fn alloc(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut::<c_void>();
    }

    let _lock = MUTEX.lock(); // lock gets dropped implicitly
    if let Some(block) = reuse_block(size) {
        get_mem_region(block)
    } else if let Some(block) = alloc_block(size) {
        get_mem_region(block)
    } else {
        ptr::null_mut::<c_void>()
    }
}

#[panic_handler]
fn panic(info: &panic::PanicInfo) -> ! {
    libc_eprintln!("panic occurred: {:?}", info);
    unsafe { intrinsics::abort() }
}

//#[lang = "panic_fmt"] extern fn panic_fmt() -> ! { unsafe { intrinsics::abort() } }
#[lang = "eh_personality"] extern fn eh_personality() {}
#[lang = "eh_unwind_resume"] extern fn eh_unwind_resume() {}
