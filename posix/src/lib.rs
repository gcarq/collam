#![feature(ptr_internals)]
#![feature(core_intrinsics)]
#![feature(lang_items)]
#![no_std]

#[macro_use]
extern crate collam;

use core::alloc::{GlobalAlloc, Layout};
use core::intrinsics::{abort, unlikely};
use core::ptr::{null_mut, Unique};
use core::{ffi::c_void, panic};

use collam::alloc::{block::BlockPtr, Collam};
use collam::MIN_ALIGN;

static COLLAM: Collam = Collam::new();

#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    let layout = Layout::from_size_align_unchecked(size, MIN_ALIGN);
    COLLAM.alloc(layout).cast::<c_void>()
}

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
    let layout = Layout::from_size_align_unchecked(total_size, MIN_ALIGN);
    COLLAM.alloc_zeroed(layout).cast::<c_void>()
}

#[no_mangle]
pub unsafe extern "C" fn realloc(p: *mut c_void, size: usize) -> *mut c_void {
    if p.is_null() {
        // If ptr is NULL, then the call is equivalent to malloc(size), for all values of size.
        let layout = Layout::from_size_align_unchecked(size, MIN_ALIGN);
        return COLLAM.alloc(layout).cast::<c_void>();
    }

    let p = p.cast::<u8>();
    let layout = Layout::from_size_align_unchecked(0, MIN_ALIGN);

    if size == 0 {
        // If size is equal to zero, and ptr is not NULL,
        // then the call is equivalent to free(ptr).
        COLLAM.dealloc(p, layout);
        null_mut()
    } else {
        COLLAM.realloc(p, layout, size).cast::<c_void>()
    }
}

#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    let layout = Layout::from_size_align_unchecked(0, MIN_ALIGN);
    COLLAM.dealloc(ptr.cast::<u8>(), layout)
}

#[no_mangle]
pub unsafe extern "C" fn malloc_usable_size(ptr: *mut c_void) -> usize {
    if ptr.is_null() {
        return 0;
    }

    // Its safe to use Unique_unchecked since we already checked for null pointers.
    let block = match BlockPtr::from_mem_region(Unique::new_unchecked(ptr.cast::<u8>())) {
        Some(b) => b,
        None => return 0,
    };
    if unlikely(!block.as_ref().verify()) {
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
#[no_mangle]
pub extern "C" fn mallopt(param: i32, value: i32) -> i32 {
    eprintln!(
        "[mallopt] not implemented! (param={}, value={})",
        param, value
    );
    1
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &panic::PanicInfo) -> ! {
    eprintln!("[libcollam.so]: panic occurred: {:?}", info);
    abort()
}

#[cfg(not(test))]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
