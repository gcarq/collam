#![feature(stmt_expr_attributes, lang_items, core_intrinsics, core_panic_info)]
#![no_std]

extern crate libc;
extern crate libc_print;
extern crate spin;

use core::ffi::c_void;
use core::{cmp, intrinsics, panic, ptr};

use libc_print::libc_eprintln;

mod macros;
mod heap;
mod meta;
mod util;

static MUTEX: spin::Mutex<()> = spin::Mutex::new(());

#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    let _lock = MUTEX.lock();
    return meta::alloc(size);
}

#[no_mangle]
pub extern "C" fn calloc(nobj: usize, size: usize) -> *mut c_void {
    let total_size = match nobj.checked_mul(size) {
        Some(x) => x,
        None => panic!("integer overflow detected (nobj={}, size={})", nobj, size),
    };

    let _lock = MUTEX.lock();
    let ptr = meta::alloc(total_size);
    // Initialize memory region with 0
    unsafe { intrinsics::volatile_set_memory(ptr, 0, total_size) }
    return ptr;
}

#[no_mangle]
pub extern "C" fn realloc(p: *mut c_void, size: usize) -> *mut c_void {
    if p.is_null() {
        // If ptr is NULL, then the call is equivalent to malloc(size), for all values of size
        let _lock = MUTEX.lock();
        return meta::alloc(size);
    } else if size == 0 {
        // if size is equal to zero, and ptr is not NULL,
        // then the call is equivalent to free(ptr)
        free(p);
        return ptr::null_mut();
    }

    let _lock = MUTEX.lock();
    let new_ptr = meta::alloc(size);
    if new_ptr == ptr::null_mut() {
        return new_ptr;
    }
    unsafe {
        let old_block = heap::get_block_meta(p);
        (*old_block).verify(true, true);
        let copy_size = cmp::min(size, (*old_block).size);
        intrinsics::volatile_copy_nonoverlapping_memory(new_ptr, p, copy_size);

        // Add old block back to heap structure
        heap::insert(old_block)
    }
    return new_ptr;
}

#[no_mangle]
pub extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    let _lock = MUTEX.lock();
    unsafe {
        let block = heap::get_block_meta(ptr);
        if !(*block).verify(false, true) {
            eprintln!("     -> {} at {:?}", *block, block);
            return;
        }
        // Add freed block back to heap structure
        debug_assert!((*block).size > 0);
        heap::insert(block);
    }
}

#[panic_handler]
fn panic(info: &panic::PanicInfo) -> ! {
    eprintln!("panic occurred: {:?}", info);
    unsafe { intrinsics::abort() };
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
#[lang = "eh_unwind_resume"]
extern "C" fn eh_unwind_resume() {}
