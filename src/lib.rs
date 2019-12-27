#![feature(core_intrinsics)]
#![feature(ptr_internals)]
#![feature(alloc_layout_extra)]
#![feature(const_fn)]
#![no_std]

#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate libc_print;
extern crate spin;

#[cfg(test)]
#[macro_use]
extern crate std;

#[allow(unused_imports)]
use libc_print::libc_eprintln;

mod macros;
pub mod alloc;
mod sources;
mod util;
