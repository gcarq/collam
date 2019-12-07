#![feature(stmt_expr_attributes)]
#![feature(lang_items)]
#![feature(core_intrinsics)]
#![feature(core_panic_info)]
#![feature(ptr_internals)]
#![feature(alloc_layout_extra)]
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
use core::{ffi::c_void, intrinsics, panic};

use crate::alloc::block::BlockPtr;
use libc_print::libc_eprintln;

mod macros;
mod alloc;
#[cfg(feature = "stats")]
mod stats;
mod util;

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    let layout = match util::pad_to_scalar(size) {
        Ok(l) => l,
        Err(_) => return null_mut(),
    };

    match unsafe { alloc::alloc(layout) } {
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

    let layout = match util::pad_to_scalar(total_size) {
        Ok(l) => l,
        Err(_) => return null_mut(),
    };

    unsafe {
        match alloc::alloc_zeroed(layout) {
            Some(p) => p.cast::<c_void>().as_ptr(),
            None => null_mut(),
        }
    }
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn realloc(p: *mut c_void, size: usize) -> *mut c_void {
    if p.is_null() {
        // If ptr is NULL, then the call is equivalent to malloc(size), for all values of size.

        let layout = match util::pad_to_scalar(size) {
            Ok(l) => l,
            Err(_) => return null_mut(),
        };

        return match unsafe { alloc::alloc(layout) } {
            Some(p) => p.as_ptr(),
            None => null_mut(),
        };
    }

    let ptr = unsafe { Unique::new_unchecked(p) };

    if size == 0 {
        // If size is equal to zero, and ptr is not NULL,
        // then the call is equivalent to free(ptr).
        unsafe { alloc::dealloc(ptr) };
        return null_mut();
    }

    match unsafe { alloc::realloc(ptr, size) } {
        Some(p) => p.as_ptr(),
        None => null_mut(),
    }
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn free(ptr: *mut c_void) {
    let ptr = match Unique::new(ptr) {
        Some(p) => p,
        None => return,
    };

    unsafe { alloc::dealloc(ptr) };
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn malloc_usable_size(ptr: *mut c_void) -> usize {
    if ptr.is_null() {
        return 0;
    }

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
