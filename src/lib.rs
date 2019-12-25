#![feature(core_intrinsics)]
#![feature(ptr_internals)]
#![feature(alloc_layout_extra)]
#![no_std]

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
#[cfg(feature = "stats")]
mod stats;
mod util;
