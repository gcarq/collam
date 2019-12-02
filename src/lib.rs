#![feature(stmt_expr_attributes)]
#![feature(lang_items)]
#![feature(core_intrinsics)]
#![feature(core_panic_info)]
#![feature(ptr_internals)]
#![no_std]

extern crate libc;
extern crate libc_print;
#[macro_use]
extern crate lazy_static;
extern crate spin;

#[cfg(test)]
#[macro_use]
extern crate std;

use core::ptr::{null_mut, Unique};
use core::{cmp, ffi::c_void, intrinsics, panic};

use crate::heap::block::BlockPtr;
use libc_print::libc_eprintln;

mod macros;
mod heap;
#[cfg(feature = "stats")]
mod stats;
mod util;

static MUTEX: spin::Mutex<()> = spin::Mutex::new(());
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    let _lock = MUTEX.lock();
    match heap::alloc(size) {
        Some(p) => p.as_ptr(),
        None => null_mut(),
    }
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn calloc(nobj: usize, size: usize) -> *mut c_void {
    let total_size = match nobj.checked_mul(size) {
        Some(x) => x,
        None => panic!("integer overflow detected (nobj={}, size={})", nobj, size),
    };

    let _lock = MUTEX.lock();
    let ptr = match heap::alloc(total_size) {
        Some(p) => p.as_ptr(),
        None => return null_mut(),
    };
    // Initialize memory region with 0.
    unsafe { intrinsics::volatile_set_memory(ptr, 0, total_size) }
    ptr
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn realloc(p: *mut c_void, size: usize) -> *mut c_void {
    if p.is_null() {
        // If ptr is NULL, then the call is equivalent to malloc(size), for all values of size.
        let _lock = MUTEX.lock();
        return match heap::alloc(size) {
            Some(p) => p.as_ptr(),
            None => null_mut(),
        };
    } else if size == 0 {
        // If size is equal to zero, and ptr is not NULL,
        // then the call is equivalent to free(ptr).
        free(p);
        return null_mut();
    }

    let mut old_block = unsafe {
        // Its safe to use Unique_unchecked since we already checked for null pointers.
        let block = match BlockPtr::from_mem_region(Unique::new_unchecked(p)) {
            Some(b) => b,
            None => return null_mut(),
        };

        if !block.verify() {
            panic!("Unable to verify {} at {:p}", block.as_ref(), block);
        }
        block
    };
    let old_block_size = old_block.size();
    let size = match util::align_scalar(size) {
        Ok(size) => size,
        Err(_) => return null_mut(),
    };

    let _lock = MUTEX.lock();
    // Shrink allocated block if size is smaller.
    if size < old_block_size {
        heap::shrink_insert_rem(&mut old_block, size);
        return p;
    }

    // Just return pointer if size didn't change.
    if size == old_block_size {
        return p;
    }

    // Allocate new region to fit size.
    let new_ptr = match heap::alloc(size) {
        Some(p) => p.as_ptr(),
        None => return null_mut(),
    };
    let copy_size = cmp::min(size, old_block_size);
    unsafe {
        intrinsics::volatile_copy_nonoverlapping_memory(new_ptr, p, copy_size);
        // Add old block back to heap structure.
        heap::insert(old_block)
    }
    new_ptr
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    let _lock = MUTEX.lock();
    unsafe {
        // Its safe to use Unique_unchecked since we already checked for null pointers.
        let block = match BlockPtr::from_mem_region(Unique::new_unchecked(ptr)) {
            Some(b) => b,
            None => return,
        };
        if !block.verify() {
            eprintln!("free(): Unable to verify {} at {:p}", block.as_ref(), block);
            return;
        }
        // Add freed block back to heap structure.
        heap::insert(block);
    }
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn malloc_usable_size(ptr: *mut c_void) -> usize {
    if ptr.is_null() {
        return 0;
    }

    let _lock = MUTEX.lock();
    // Its safe to use Unique_unchecked since we already checked for null pointers.
    let block = match BlockPtr::from_mem_region(unsafe { Unique::new_unchecked(ptr) }) {
        Some(b) => b,
        None => return 0,
    };
    if !block.verify() {
        eprintln!(
            "malloc_usable_size(): Unable to verify {} at {:p}",
            block.as_ref(),
            block
        );
        return 0;
    }
    block.size()
}

// TODO: implement me
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn mallopt(param: i32, value: i32) -> i32 {
    eprintln!(
        "[mallopt] not implemented! (param={}, value={})",
        param, value
    );
    return 1;
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &panic::PanicInfo) -> ! {
    eprintln!("panic occurred: {:?}", info);
    unsafe { intrinsics::abort() };
}

#[cfg(not(test))]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[cfg(not(test))]
#[lang = "eh_unwind_resume"]
extern "C" fn eh_unwind_resume() {}
