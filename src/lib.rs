#![feature(core_intrinsics)]
#![feature(ptr_internals)]
#![no_std]

//#![warn(clippy::pedantic)]

#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate libc_print;

#[cfg(test)]
#[macro_use]
extern crate std;

#[allow(unused_imports)]
use libc_print::libc_eprintln;

mod macros;

pub mod alloc;
mod sources;
mod util;

#[cfg(all(any(
    target_arch = "arm",
    target_arch = "mips",
    target_arch = "mipsel",
    target_arch = "powerpc"
)))]
pub const MIN_ALIGN: usize = 8;
#[cfg(all(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "powerpc64",
    target_arch = "powerpc64le",
    target_arch = "mips64",
    target_arch = "s390x",
    target_arch = "sparc64"
)))]
pub const MIN_ALIGN: usize = 16;
