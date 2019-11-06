#![feature(stmt_expr_attributes, lang_items, core_intrinsics, core_panic_info)]
#![no_std]

extern crate libc;
extern crate libc_print;
extern crate spin;

use core::ffi::c_void;
use core::{cmp, intrinsics, panic, ptr};

use libc_print::libc_eprintln;

use crate::heap::get_block_meta;
use crate::meta::alloc;

mod macros;
mod heap;
mod meta;
mod util;

static MUTEX: spin::Mutex<()> = spin::Mutex::new(());

#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    return alloc(size);
}

#[no_mangle]
pub extern "C" fn calloc(nobj: usize, size: usize) -> *mut c_void {
    let total_size = match nobj.checked_mul(size) {
        Some(x) => x,
        None => panic!("integer overflow detected (nobj={}, size={})", nobj, size),
    };
    let pointer = alloc(total_size);
    let _lock = MUTEX.lock(); // lock gets dropped implicitly
    unsafe { pointer.write_bytes(0, total_size) }
    pointer
}

#[no_mangle]
pub extern "C" fn realloc(p: *mut c_void, size: usize) -> *mut c_void {
    if p.is_null() {
        // If ptr is NULL, then the call is equivalent to malloc(size), for all values of size
        return alloc(size);
    } else if size == 0 {
        // if size is equal to zero, and ptr is not NULL,
        // then the call is equivalent to free(ptr)
        free(p);
        return ptr::null_mut();
    }

    let new_ptr = alloc(size);
    let lock = MUTEX.lock();
    unsafe {
        let old_block = get_block_meta(p);
        let new_blk = get_block_meta(new_ptr);
        let cpy_size = cmp::min(size, (*old_block).size);
        libc_eprintln!(
            "[realloc] Copying {} bytes from {:?} to {:?}...",
            cpy_size,
            old_block,
            new_blk
        );
        libc_eprintln!("    from -> {} at {:?}", *old_block, old_block);
        libc_eprintln!("      to -> {} at {:?}", *new_blk, new_blk);
        ptr::copy(p, new_ptr, cpy_size);
    }
    drop(lock);

    free(p);
    new_ptr
}

#[no_mangle]
pub extern "C" fn free(pointer: *mut c_void) {
    if pointer.is_null() {
        return;
    }

    let _lock = MUTEX.lock(); // lock gets dropped implicitly
    let block = unsafe { get_block_meta(pointer) };
    // Re-add block to list
    heap::insert(block);
}

#[panic_handler]
fn panic(info: &panic::PanicInfo) -> ! {
    libc_eprintln!("panic occurred: {:?}", info);
    unsafe { intrinsics::abort() }
}

//#[lang = "panic_fmt"] extern fn panic_fmt() -> ! { unsafe { intrinsics::abort() } }
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
#[lang = "eh_unwind_resume"]
extern "C" fn eh_unwind_resume() {}
