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

use core::intrinsics::unlikely;
use core::ptr::{null_mut, Unique};
use core::{alloc::GlobalAlloc, ffi::c_void, intrinsics, panic};

use libc_print::libc_eprintln;

use crate::alloc::{block::BlockPtr, Collam};
use core::alloc::Layout;

mod macros;
mod alloc;
#[cfg(feature = "stats")]
mod stats;
mod util;

static mut COLLAM: Collam = Collam::new();

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    let layout = match util::pad_to_scalar(size) {
        Ok(l) => l,
        Err(_) => return null_mut(),
    };

    COLLAM.alloc(layout).cast::<c_void>()
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn calloc(nobj: usize, size: usize) -> *mut c_void {
    let total_size = match nobj.checked_mul(size) {
        Some(x) => x,
        None => {
            eprintln!(
                "integer overflow detected for calloc(nobj={}, size={})",
                nobj, size
            );
            return null_mut();
        }
    };

    let layout = match util::pad_to_scalar(total_size) {
        Ok(l) => l,
        Err(_) => return null_mut(),
    };

    COLLAM.alloc_zeroed(layout).cast::<c_void>()
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn realloc(p: *mut c_void, size: usize) -> *mut c_void {
    if p.is_null() {
        // If ptr is NULL, then the call is equivalent to malloc(size), for all values of size.
        return match util::pad_to_scalar(size) {
            Ok(layout) => COLLAM.alloc(layout).cast::<c_void>(),
            Err(_) => null_mut(),
        };
    }

    let p = p.cast::<u8>();
    if size == 0 {
        // If size is equal to zero, and ptr is not NULL,
        // then the call is equivalent to free(ptr).
        let layout = Layout::from_size_align_unchecked(0, 16);
        COLLAM.dealloc(p, layout);
        null_mut()
    } else {
        let layout = Layout::from_size_align_unchecked(0, 16);
        COLLAM.realloc(p, layout, size).cast::<c_void>()
    }
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    let layout = Layout::from_size_align_unchecked(0, 16);
    COLLAM.dealloc(ptr.cast::<u8>(), layout)
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn malloc_usable_size(ptr: *mut c_void) -> usize {
    if ptr.is_null() {
        return 0;
    }

    // Its safe to use Unique_unchecked since we already checked for null pointers.
    let block = match BlockPtr::from_mem_region(Unique::new_unchecked(ptr)) {
        Some(b) => b,
        None => return 0,
    };
    if unlikely(!block.verify()) {
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
