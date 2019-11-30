use core::ffi::c_void;
use core::ptr::Unique;

use crate::heap::block::BlockPtr;
use libc_print::libc_eprintln;

static mut HEAP_LOW_ADDR: Option<Unique<c_void>> = None;
static mut HEAP_HIGH_ADDR: Option<Unique<c_void>> = None;

static mut HEAP_HEAD: Option<BlockPtr> = None;
static mut HEAP_TAIL: Option<BlockPtr> = None;

pub unsafe fn update_ends(head: Option<BlockPtr>, tail: Option<BlockPtr>) {
    HEAP_HEAD = head;
    HEAP_TAIL = tail;
}

/// Updates heap information.
/// Should only be called with the current program break.
pub unsafe fn update_heap_info(ptr: *mut c_void) {
    if HEAP_LOW_ADDR.is_none() {
        HEAP_LOW_ADDR = Unique::new(ptr);
    }

    match HEAP_HIGH_ADDR {
        Some(addr) => {
            if addr.as_ptr() < ptr {
                HEAP_HIGH_ADDR = Unique::new(ptr)
            }
        }
        None => HEAP_HIGH_ADDR = Unique::new(ptr),
    }
}

pub unsafe fn print() {
    if HEAP_HEAD.is_some() && HEAP_TAIL.is_some() {
        let head = HEAP_HEAD.unwrap();
        let tail = HEAP_TAIL.unwrap();
        println!("[stats]: head: {} at\t{:p}", head.as_ref(), head);
        println!("[stats]: tail: {} at\t{:p}", tail.as_ref(), tail);
        println!(
            "[stats]: difference between head and tail: {} bytes",
            tail.as_ptr() as usize - head.as_ptr() as usize
        );
    }

    if HEAP_LOW_ADDR.is_some() && HEAP_HIGH_ADDR.is_some() {
        println!(
            "[stats]: total heap size: {} bytes\n",
            HEAP_HIGH_ADDR.unwrap().as_ptr() as usize - HEAP_LOW_ADDR.unwrap().as_ptr() as usize
        );
    }
}
