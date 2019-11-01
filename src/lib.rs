#![feature(lang_items, core_intrinsics)]
#![no_std]

extern crate libc;
extern crate libc_print;

use core::panic::PanicInfo;

use libc_print::libc_eprintln;
use core::ffi::c_void;
use core::{cmp, intrinsics, ptr};

use crate::meta::{alloc_block, get_block_meta, reuse_block, get_mem_region};
use crate::mutex::Mutex;

mod macros;
mod meta;
mod util;
mod mutex;

pub static MUTEX: Mutex = Mutex::new();

#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    return alloc(size);
}

#[no_mangle]
pub extern "C" fn calloc(nobj: usize, size: usize) -> *mut c_void {
    // TODO: check for int overflow
    let total_size = nobj * size;
    let pointer = alloc(total_size);
    MUTEX.lock();
    unsafe {
        pointer.write_bytes(0, total_size);
    }
    MUTEX.unlock();
    pointer
}

#[no_mangle]
pub extern "C" fn realloc(p: *mut c_void, size: usize) -> *mut c_void {
    let new_ptr = alloc(size);
    // If passed pointer is NULL, just return newly allocated ptr
    if p.is_null() {
        return new_ptr;
    }

    let old_block = get_block_meta(p);
    MUTEX.lock();
    unsafe {
        // TODO: don't reuse blocks and use copy_nonoverlapping
        let copy_size = cmp::min(size, (*old_block).size);
        new_ptr.copy_from(p, copy_size);
        for i in 0..copy_size {
            assert_eq!(*(p as *mut u8).offset(i as isize), *(new_ptr as *mut u8).offset(i as isize));
        }
    }
    MUTEX.unlock();
    free(p);
    //libc_eprintln!("[libdmalloc.so] realloc: reallocated {} bytes at {:?}\n", size, p);
    new_ptr
}

#[no_mangle]
pub extern "C" fn free(pointer: *mut c_void) {
    //libc_eprintln!("[libdmalloc.so] free: dropping {:?}\n", pointer);
    if pointer.is_null() {
        return
    }

    MUTEX.lock();
    let block = get_block_meta(pointer);
    unsafe {
        assert_eq!((*block).unused, false, "{} at {:?}", *block, block);
        (*block).unused = true;
    }
    MUTEX.unlock();
}

fn alloc(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut::<c_void>();
    }

    MUTEX.lock();
    let pointer = if let Some(block) = reuse_block(size) {
        get_mem_region(block)
    } else if let Some(block) = alloc_block(size) {
        get_mem_region(block)
    } else {
        ptr::null_mut::<c_void>()
    };
    MUTEX.unlock();
    return pointer;
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    libc_eprintln!("panic occurred: {:?}", info);
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