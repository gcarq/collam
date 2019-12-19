#![feature(stmt_expr_attributes)]
#![feature(lang_items)]
#![feature(core_intrinsics)]
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

use core::{intrinsics, panic};

use libc_print::libc_eprintln;

mod macros;
pub mod alloc;
#[cfg(all(feature = "posix", not(test)))]
pub mod posix;
#[cfg(feature = "stats")]
mod stats;
mod util;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &panic::PanicInfo) -> ! {
    eprintln!("[libcollam.so]: panic occurred: {:?}", info);
    unsafe { intrinsics::abort() };
}

#[cfg(not(test))]
#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[cfg(not(test))]
#[lang = "eh_unwind_resume"]
extern "C" fn eh_unwind_resume() {}
