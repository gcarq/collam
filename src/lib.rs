#![feature(stmt_expr_attributes, lang_items, core_intrinsics, core_panic_info)]
#![no_std]

extern crate libc;
extern crate libc_print;
extern crate spin;

use core::ffi::c_void;
use core::{cmp, intrinsics, panic};

use libc_print::libc_eprintln;

use crate::heap::list::get_block_meta;
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
    let new_ptr = alloc(size);
    // If passed pointer is NULL, just return newly allocated ptr
    if p.is_null() {
        return new_ptr;
    }

    let old_block = get_block_meta(p);
    // TODO: don't reuse blocks and use copy_nonoverlapping
    let lock = MUTEX.lock();
    unsafe { new_ptr.copy_from(p, cmp::min(size, (*old_block).size)) }
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
    let block = get_block_meta(pointer);
    // Re-add block to list
    heap::insert(block);
}

#[panic_handler]
fn panic(info: &panic::PanicInfo) -> ! {
    log!("panic occurred: {:?}", info);
    unsafe { intrinsics::abort() }
}

//#[lang = "panic_fmt"] extern fn panic_fmt() -> ! { unsafe { intrinsics::abort() } }
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
#[lang = "eh_unwind_resume"]
extern "C" fn eh_unwind_resume() {}
