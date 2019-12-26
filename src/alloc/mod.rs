use core::alloc::{GlobalAlloc, Layout};
use core::{cmp, ffi::c_void, intrinsics, intrinsics::unlikely, ptr::null_mut, ptr::Unique};

use libc_print::libc_eprintln;
use spin::Mutex;

use crate::alloc::block::{BlockPtr, BLOCK_MIN_REGION_SIZE};
use crate::alloc::heap::Heap;
use crate::util;

pub mod block;
mod heap;
mod list;

lazy_static! {
    static ref HEAP: Mutex<Heap> = spin::Mutex::new(Heap::new());
}

pub struct Collam;

impl Collam {
    /// Requests and returns suitable empty `BlockPtr`.
    unsafe fn request_block(&self, size: usize) -> Option<BlockPtr> {
        let mut heap = HEAP.lock();
        (*heap).request(size)
    }

    /// Releases the given `BlockPtr` back to the allocator.
    unsafe fn release_block(&self, block: BlockPtr) {
        let mut heap = HEAP.lock();
        (*heap).release(block)
    }
}

unsafe impl GlobalAlloc for Collam {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return null_mut();
        }

        let layout = match util::pad_to_scalar(layout.size()) {
            Ok(l) => l,
            Err(_) => return null_mut(),
        };

        let size = cmp::max(layout.size(), BLOCK_MIN_REGION_SIZE);
        dprintln!("[libcollam.so]: alloc(size={})", size);
        let mut block = match self.request_block(size) {
            Some(b) => b,
            None => {
                dprintln!("[libcollam.so]: failed for size: {}\n", layout.size());
                return null_mut();
            }
        };

        if let Some(rem_block) = block.shrink(size) {
            self.release_block(rem_block);
        }

        dprintln!(
            "[libcollam.so]: returning {} at {:p}\n",
            block.as_ref(),
            block
        );
        debug_assert!(
            block.size() >= size,
            "requested_size={}, got_block={}",
            size,
            block.as_ref()
        );
        block.mem_region().cast::<u8>().as_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if let Some(p) = Unique::new(ptr) {
            dprintln!("[libcollam.so]: dealloc(ptr={:p})", ptr);

            let block = match BlockPtr::from_mem_region(p.cast::<c_void>()) {
                Some(b) => b,
                None => return,
            };
            if unlikely(!block.as_ref().verify()) {
                eprintln!("free(): Unable to verify {} at {:p}", block.as_ref(), block);
                return;
            }
            // Add freed block back to heap structure.
            self.release_block(block)
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        let ptr = match Unique::new(ptr) {
            Some(p) => p.cast::<c_void>(),
            None => return null_mut(),
        };

        dprintln!("[libcollam.so]: realloc(ptr={:p}, size={})", ptr, new_size);

        // FIXME: Alignment  to old layout needed?
        let new_layout = match util::pad_to_scalar(new_size) {
            Ok(l) => l,
            Err(_) => return null_mut(),
        };

        let mut old_block = match BlockPtr::from_mem_region(ptr) {
            Some(b) => b,
            None => return null_mut(),
        };

        if unlikely(!old_block.as_ref().verify()) {
            eprintln!(
                "realloc(): Unable to verify {} at {:p}",
                old_block.as_ref(),
                old_block
            );
            return null_mut();
        }

        if new_layout.size() == old_block.size() {
            // Just return pointer if size didn't change.
            ptr.cast::<u8>().as_ptr()
        } else if new_layout.size() > old_block.size() {
            // Allocate new region to fit size.
            let new_ptr = self.alloc(new_layout).cast::<c_void>();
            let copy_size = cmp::min(new_layout.size(), old_block.size());
            intrinsics::volatile_copy_nonoverlapping_memory(new_ptr, ptr.as_ptr(), copy_size);
            // Add old block back to heap structure.
            self.release_block(old_block);
            new_ptr.cast::<u8>()
        } else {
            // Shrink allocated block if size is smaller.
            let size = cmp::max(new_layout.size(), BLOCK_MIN_REGION_SIZE);
            if let Some(rem_block) = old_block.shrink(size) {
                self.release_block(rem_block);
            }
            ptr.cast::<u8>().as_ptr()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc::block::BLOCK_META_SIZE;
    use crate::util;
    use core::intrinsics::write_bytes;

    #[test]
    fn test_collam_alloc_ok() {
        unsafe {
            let layout = util::pad_to_scalar(123).expect("unable to align layout");
            let ptr = Collam.alloc(layout);
            assert!(!ptr.is_null());
            write_bytes(ptr, 1, 123);
            Collam.dealloc(ptr, layout);
        }
    }

    #[test]
    fn test_collam_alloc_zero_size() {
        unsafe {
            let layout = util::pad_to_scalar(0).expect("unable to align layout");
            let ptr = Collam.alloc(layout);
            assert!(ptr.is_null());
        }
    }

    #[test]
    fn test_collam_realloc_bigger_size() {
        unsafe {
            let layout = util::pad_to_scalar(16).expect("unable to align layout");
            let ptr = Collam.alloc(layout);
            assert!(!ptr.is_null());

            let ptr = Collam.realloc(ptr, layout, 789);
            write_bytes(ptr, 2, 789);
            Collam.dealloc(ptr, layout);
        }
    }

    #[test]
    fn test_collam_realloc_smaller_size() {
        unsafe {
            let layout = util::pad_to_scalar(512).expect("unable to align layout");
            let ptr = Collam.alloc(layout);
            assert!(!ptr.is_null());

            let ptr = Collam.realloc(ptr, layout, 128);
            write_bytes(ptr, 2, 128);
            Collam.dealloc(ptr, layout);
        }
    }

    #[test]
    fn test_collam_realloc_same_size() {
        unsafe {
            let layout = util::pad_to_scalar(512).expect("unable to align layout");
            let ptr = Collam.alloc(layout);
            assert!(!ptr.is_null());
            let ptr2 = Collam.realloc(ptr, layout, 512);
            assert!(!ptr2.is_null());
            assert_eq!(ptr, ptr2);
            Collam.dealloc(ptr, layout);
        }
    }

    #[test]
    fn test_collam_realloc_null() {
        unsafe {
            let layout = util::pad_to_scalar(16).expect("unable to align layout");
            let ptr = Collam.realloc(null_mut(), layout, 789);
            assert_eq!(ptr, null_mut());
        }
    }

    #[test]
    fn test_collam_dealloc_null() {
        unsafe {
            let layout = util::pad_to_scalar(16).expect("unable to align layout");
            Collam.dealloc(null_mut(), layout);
        }
    }

    #[test]
    fn test_collam_realloc_memory_corruption() {
        unsafe {
            let layout = util::pad_to_scalar(16).expect("unable to align layout");
            let ptr = Collam.alloc(layout);
            assert!(!ptr.is_null());

            // Overwrite block metadata to simulate memory corruption
            let meta_ptr = ptr.sub(BLOCK_META_SIZE);
            meta_ptr.write_bytes(0, BLOCK_META_SIZE);

            // Calling realloc on a corrupt memory region
            let ptr = Collam.realloc(ptr, layout, 789);
            assert!(ptr.is_null());

            // Calling alloc again. We expect to get a new block, the old memory is leaked.
            let ptr = Collam.alloc(layout);
            assert!(!ptr.is_null());
            Collam.dealloc(ptr, layout);
        }
    }

    #[test]
    fn test_collam_dealloc_memory_corruption() {
        unsafe {
            let layout = util::pad_to_scalar(32).expect("unable to align layout");
            let ptr = Collam.alloc(layout);
            assert!(!ptr.is_null());

            // Overwrite block metadata to simulate memory corruption
            let meta_ptr = ptr.sub(BLOCK_META_SIZE);
            meta_ptr.write_bytes(0, BLOCK_META_SIZE);
            Collam.dealloc(ptr, layout);
        }
    }
}
