// Enable std and main generation for tests
#![cfg_attr(not(test), no_std)]
//#![cfg_attr(not(test), no_main)]

//#![feature(libc)]
#![feature(core_intrinsics)]

extern crate libc;
extern crate libc_print;

use core::panic::PanicInfo;

use libc::{size_t, c_void};
use libc_print::libc_println;
use crate::meta::alloc_block;
use core::intrinsics;

mod macros;
mod meta;


#[no_mangle]
pub extern "C" fn malloc(size: size_t) -> *mut c_void {
    if size <= 0 {
        return 0 as *mut c_void;
    }
    let block = alloc_block(size);
    let ptr = unsafe { block.offset(1) } as *mut c_void;
    libc_println!("[libdmalloc.so] malloc: allocated {} bytes at {:?}", size, ptr);
    ptr
}

#[no_mangle]
pub extern "C" fn calloc(nobj: size_t, size: size_t) -> *mut c_void {
    let ptr = unsafe { libc::sbrk( 0) };
    let req = unsafe { libc::sbrk(size as isize) };
    libc_println!("[libdmalloc.so] calloc: allocatd {} bytes at {:?}\n", size, ptr);
    ptr
}

#[no_mangle]
pub extern "C" fn realloc(p: *mut c_void, size: size_t) -> *mut c_void {
    libc_println!("[libdmalloc.so] realloc: reallocated {} bytes at {:?}\n", size, p);
    let ptr = unsafe { libc::sbrk(size as isize) };
    p
}

#[no_mangle]
pub extern "C" fn free(p: *mut c_void) {
    libc_println!("[libdmalloc.so] free: dropping {:?}\n", p);
}

#[cfg(not(test))] // only compile when the test flag is not set
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe { intrinsics::abort() }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
