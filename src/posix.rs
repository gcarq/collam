use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{null_mut, Unique};
use core::{ffi::c_void, intrinsics::unlikely, panic};

use libc_print::libc_eprintln;

use crate::alloc::{block::BlockPtr, Collam};
use crate::util;

static mut COLLAM: Collam = Collam::new();

#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    let layout = match util::pad_to_scalar(size) {
        Ok(l) => l,
        Err(_) => return null_mut(),
    };

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

    let layout = match util::pad_to_scalar(total_size) {
        Ok(l) => l,
        Err(_) => return null_mut(),
    };

    COLLAM.alloc_zeroed(layout).cast::<c_void>()
}

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

#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    let layout = Layout::from_size_align_unchecked(0, 16);
    COLLAM.dealloc(ptr.cast::<u8>(), layout)
}

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
#[no_mangle]
pub extern "C" fn mallopt(param: i32, value: i32) -> i32 {
    eprintln!(
        "[mallopt] not implemented! (param={}, value={})",
        param, value
    );
    return 1;
}
