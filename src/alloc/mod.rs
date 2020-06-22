use core::alloc::{GlobalAlloc, Layout};
use core::{cmp, intrinsics, ptr::null_mut, ptr::Unique};

use libc_print::libc_eprintln;
use spin::Mutex;

use crate::alloc::arena::heap::HeapArena;
use crate::alloc::block::{BlockPtr, BLOCK_MIN_REGION_SIZE};
use crate::util;

mod arena;
pub mod block;
mod list;

pub struct Collam {
    heap: Mutex<HeapArena>,
}

impl Collam {
    #[must_use]
    pub fn new() -> Self {
        Self {
            heap: spin::Mutex::new(HeapArena::new()),
        }
    }

    /// Requests and returns suitable empty `BlockPtr`.
    #[inline]
    fn request_block(&self, size: usize) -> Option<BlockPtr> {
        // SAFETY: we know it is thread safe, because we're locking the mutex
        unsafe { self.heap.lock().request(size) }
    }

    /// Releases the given `BlockPtr` back to the allocator.
    #[inline]
    fn release_block(&self, block: BlockPtr) {
        // SAFETY: we know it is thread safe, because we're locking the mutex
        unsafe { self.heap.lock().release(block) }
    }
}

unsafe impl GlobalAlloc for Collam {
    /// Allocate memory as described by the given `layout`.
    ///
    /// Returns a pointer to newly-allocated memory,
    /// or null to indicate allocation failure.
    ///
    /// # Safety
    ///
    /// This function is unsafe because undefined behavior can result
    /// if the caller does not ensure that `layout` has non-zero size.
    ///
    /// (Extension subtraits might provide more specific bounds on
    /// behavior, e.g., guarantee a sentinel address or a null pointer
    /// in response to a zero-size allocation request.)
    ///
    /// The allocated block of memory may or may not be initialized.
    ///
    /// # Errors
    ///
    /// Returning a null pointer indicates that either memory is exhausted
    /// or `layout` does not meet this allocator's size or alignment constraints.
    ///
    /// Implementations are encouraged to return null on memory
    /// exhaustion rather than aborting, but this is not
    /// a strict requirement. (Specifically: it is *legal* to
    /// implement this trait atop an underlying native allocation
    /// library that aborts on memory exhaustion.)
    ///
    /// Clients wishing to abort computation in response to an
    /// allocation error are encouraged to call the [`handle_alloc_error`] function,
    /// rather than directly invoking `panic!` or similar.
    ///
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return null_mut();
        }

        let layout = match util::pad_min_align(layout.size()) {
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
        block.mem_region().as_ptr()
    }

    /// Deallocate the block of memory at the given `ptr` pointer with the given `layout`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because undefined behavior can result
    /// if the caller does not ensure all of the following:
    ///
    /// * `ptr` must denote a block of memory currently allocated via
    ///   this allocator,
    ///
    /// * `layout` must be the same layout that was used
    ///   to allocate that block of memory,
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if let Some(p) = Unique::new(ptr) {
            dprintln!("[libcollam.so]: dealloc(ptr={:p})", ptr);

            let block = match BlockPtr::from_mem_region(p) {
                Some(b) => b,
                None => return,
            };
            if !block.as_ref().verify() {
                eprintln!("free(): Unable to verify {} at {:p}", block.as_ref(), block);
                return;
            }
            // Add freed block back to heap structure.
            self.release_block(block)
        }
    }

    /// Shrink or grow a block of memory to the given `new_size`.
    /// The block is described by the given `ptr` pointer and `layout`.
    ///
    /// If this returns a non-null pointer, then ownership of the memory block
    /// referenced by `ptr` has been transferred to this allocator.
    /// The memory may or may not have been deallocated,
    /// and should be considered unusable (unless of course it was
    /// transferred back to the caller again via the return value of
    /// this method). The new memory block is allocated with `layout`, but
    /// with the `size` updated to `new_size`.
    ///
    /// If this method returns null, then ownership of the memory
    /// block has not been transferred to this allocator, and the
    /// contents of the memory block are unaltered.
    ///
    /// # Safety
    ///
    /// This function is unsafe because undefined behavior can result
    /// if the caller does not ensure all of the following:
    ///
    /// * `ptr` must be currently allocated via this allocator,
    ///
    /// * `layout` must be the same layout that was used
    ///   to allocate that block of memory,
    ///
    /// * `new_size` must be greater than zero.
    ///
    /// * `new_size`, when rounded up to the nearest multiple of `layout.align()`,
    ///   must not overflow (i.e., the rounded value must be less than `usize::MAX`).
    ///
    /// (Extension subtraits might provide more specific bounds on
    /// behavior, e.g., guarantee a sentinel address or a null pointer
    /// in response to a zero-size allocation request.)
    ///
    /// # Errors
    ///
    /// Returns null if the new layout does not meet the size
    /// and alignment constraints of the allocator, or if reallocation
    /// otherwise fails.
    ///
    /// Implementations are encouraged to return null on memory
    /// exhaustion rather than panicking or aborting, but this is not
    /// a strict requirement. (Specifically: it is *legal* to
    /// implement this trait atop an underlying native allocation
    /// library that aborts on memory exhaustion.)
    ///
    /// Clients wishing to abort computation in response to a
    /// reallocation error are encouraged to call the [`handle_alloc_error`] function,
    /// rather than directly invoking `panic!` or similar.
    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        let ptr = match Unique::new(ptr) {
            Some(p) => p,
            None => return null_mut(),
        };

        dprintln!("[libcollam.so]: realloc(ptr={:p}, size={})", ptr, new_size);

        // FIXME: Alignment  to old layout needed?
        let new_layout = match util::pad_min_align(new_size) {
            Ok(l) => l,
            Err(_) => return null_mut(),
        };

        let mut old_block = match BlockPtr::from_mem_region(ptr) {
            Some(b) => b,
            None => return null_mut(),
        };

        if !old_block.as_ref().verify() {
            eprintln!(
                "realloc(): Unable to verify {} at {:p}",
                old_block.as_ref(),
                old_block
            );
            return null_mut();
        }

        match new_layout.size().cmp(&old_block.size()) {
            cmp::Ordering::Equal => {
                // Just return pointer if size didn't change.
                ptr.as_ptr()
            }
            cmp::Ordering::Greater => {
                // Allocate new region to fit size.
                let new_ptr = self.alloc(new_layout);
                let copy_size = cmp::min(new_layout.size(), old_block.size());
                intrinsics::volatile_copy_nonoverlapping_memory(new_ptr, ptr.as_ptr(), copy_size);
                // Add old block back to heap structure.
                self.release_block(old_block);
                new_ptr
            }
            cmp::Ordering::Less => {
                // Shrink allocated block if size is smaller.
                let size = cmp::max(new_layout.size(), BLOCK_MIN_REGION_SIZE);
                if let Some(rem_block) = old_block.shrink(size) {
                    self.release_block(rem_block);
                }
                ptr.as_ptr()
            }
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
            let collam = Collam::new();
            let layout = util::pad_min_align(123).expect("unable to align layout");
            let ptr = collam.alloc(layout);
            assert!(!ptr.is_null());
            write_bytes(ptr, 1, 123);
            collam.dealloc(ptr, layout);
        }
    }

    #[test]
    fn test_collam_alloc_zero_size() {
        unsafe {
            let collam = Collam::new();
            let layout = util::pad_min_align(0).expect("unable to align layout");
            let ptr = collam.alloc(layout);
            assert!(ptr.is_null());
        }
    }

    #[test]
    fn test_collam_realloc_bigger_size() {
        unsafe {
            let collam = Collam::new();
            let layout = util::pad_min_align(16).expect("unable to align layout");
            let ptr = collam.alloc(layout);
            assert!(!ptr.is_null());

            let ptr = collam.realloc(ptr, layout, 789);
            write_bytes(ptr, 2, 789);
            collam.dealloc(ptr, layout);
        }
    }

    #[test]
    fn test_collam_realloc_smaller_size() {
        unsafe {
            let collam = Collam::new();
            let layout = util::pad_min_align(512).expect("unable to align layout");
            let ptr = collam.alloc(layout);
            assert!(!ptr.is_null());

            let ptr = collam.realloc(ptr, layout, 128);
            write_bytes(ptr, 2, 128);
            collam.dealloc(ptr, layout);
        }
    }

    #[test]
    fn test_collam_realloc_same_size() {
        unsafe {
            let collam = Collam::new();
            let layout = util::pad_min_align(512).expect("unable to align layout");
            let ptr = collam.alloc(layout);
            assert!(!ptr.is_null());
            let ptr2 = collam.realloc(ptr, layout, 512);
            assert!(!ptr2.is_null());
            assert_eq!(ptr, ptr2);
            collam.dealloc(ptr, layout);
        }
    }

    #[test]
    fn test_collam_realloc_null() {
        unsafe {
            let collam = Collam::new();
            let layout = util::pad_min_align(16).expect("unable to align layout");
            let ptr = collam.realloc(null_mut(), layout, 789);
            assert_eq!(ptr, null_mut());
        }
    }

    #[test]
    fn test_collam_dealloc_null() {
        unsafe {
            let collam = Collam::new();
            let layout = util::pad_min_align(16).expect("unable to align layout");
            collam.dealloc(null_mut(), layout);
        }
    }

    #[test]
    fn test_collam_realloc_memory_corruption() {
        unsafe {
            let collam = Collam::new();
            let layout = util::pad_min_align(16).expect("unable to align layout");
            let ptr = collam.alloc(layout);
            assert!(!ptr.is_null());

            // Overwrite block metadata to simulate memory corruption
            let meta_ptr = ptr.sub(BLOCK_META_SIZE);
            meta_ptr.write_bytes(0, BLOCK_META_SIZE);

            // Calling realloc on a corrupt memory region
            let ptr = collam.realloc(ptr, layout, 789);
            assert!(ptr.is_null());

            // Calling alloc again. We expect to get a new block, the old memory is leaked.
            let ptr = collam.alloc(layout);
            assert!(!ptr.is_null());
            collam.dealloc(ptr, layout);
        }
    }

    #[test]
    fn test_collam_dealloc_memory_corruption() {
        unsafe {
            let collam = Collam::new();
            let layout = util::pad_min_align(32).expect("unable to align layout");
            let ptr = collam.alloc(layout);
            assert!(!ptr.is_null());

            // Overwrite block metadata to simulate memory corruption
            let meta_ptr = ptr.sub(BLOCK_META_SIZE);
            meta_ptr.write_bytes(0, BLOCK_META_SIZE);
            collam.dealloc(ptr, layout);
        }
    }
}
