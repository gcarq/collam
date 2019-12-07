use core::alloc::{GlobalAlloc, Layout};
use core::{cmp, ffi::c_void, intrinsics, ptr::Unique};

use libc_print::libc_eprintln;

use crate::alloc::block::{BlockPtr, BLOCK_META_SIZE};
use crate::alloc::list::IntrusiveList;
#[cfg(feature = "stats")]
use crate::stats;
use crate::util;
use core::ptr::null_mut;

pub mod block;
mod list;

lazy_static! {
    static ref PAGE_SIZE: usize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
}

pub struct Collam {
    heap: spin::Mutex<IntrusiveList>,
}

impl Collam {
    pub const fn new() -> Self {
        Collam {
            heap: spin::Mutex::new(IntrusiveList::new()),
        }
    }

    /// Inserts a `BlockPtr` to the heap structure.
    /// NOTE: The memory is returned to the OS if it is adjacent to program break.
    unsafe fn insert(&self, block: BlockPtr) {
        // Lock heap for the whole function
        let mut heap = self.heap.lock();

        #[cfg(feature = "debug")]
        {
            (*heap).debug();
        }
        #[cfg(feature = "stats")]
        {
            stats::update_ends((*heap).head, (*heap).tail);
            stats::print();
        }

        let ptr = block.next_potential_block();
        if let Some(brk) = util::sbrk(0) {
            if ptr.as_ptr() == brk.as_ptr() {
                let offset = block.block_size() as isize;
                dprintln!(
                    "[insert]: freeing {} bytes from process (break={:?})",
                    offset,
                    ptr
                );
                util::sbrk(-offset);
                return;
            }
        }

        dprintln!("[insert]: {} at {:p}", block.as_ref(), block);
        if (*heap).insert(block).is_err() {
            eprintln!("double free detected for ptr {:?}", block.mem_region());
        }
    }

    /// Removes and returns a suitable empty `BlockPtr` from the heap structure.
    #[inline]
    unsafe fn pop(&self, size: usize) -> Option<BlockPtr> {
        // Lock heap for the whole function
        let mut heap = self.heap.lock();
        let block = (*heap).pop(size)?;
        dprintln!("[pop]: {} at {:p}", block.as_ref(), block);
        Some(block)
    }

    /// Find a usable memory region for the given size either by
    /// reusing or requesting memory from the kernel.
    /// Returns a `Unique<c_void>` pointer to the memory region.
    pub unsafe fn _alloc(&self, layout: Layout) -> Option<Unique<c_void>> {
        if layout.size() == 0 {
            return None;
        }

        debug_assert_eq!(
            layout.size(),
            layout.pad_to_align().expect("unable to align").size()
        );

        dprintln!("[libcollam.so]: alloc(size={})", layout.size());

        // Check if there is already a suitable block allocated
        let mut block = if let Some(block) = self.pop(layout.size()) {
            block
        // Request new block from kernel
        } else if let Some(block) = request_block(layout.size()) {
            block
        } else {
            dprintln!("[libcollam.so]: failed for size: {}\n", layout.size());
            return None;
        };

        if let Some(rem_block) = block.shrink(layout.size()) {
            self.insert(rem_block);
        }

        dprintln!(
            "[libcollam.so]: returning {} at {:p}\n",
            block.as_ref(),
            block
        );
        debug_assert!(
            block.size() >= layout.size(),
            "requested_size={}, got_block={}",
            layout.size(),
            block.as_ref()
        );
        Some(block.mem_region())
    }

    #[inline]
    pub unsafe fn dealloc_unchecked(&self, ptr: Unique<c_void>) {
        let block = match BlockPtr::from_mem_region(ptr) {
            Some(b) => b,
            None => return,
        };
        if !block.verify() {
            eprintln!("free(): Unable to verify {} at {:p}", block.as_ref(), block);
            return;
        }
        // Add freed block back to heap structure.
        self.insert(block)
    }

    pub unsafe fn _realloc(&self, ptr: Unique<c_void>, new_size: usize) -> Option<Unique<c_void>> {
        // Align to old layout, FIXME: needed?
        //let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let new_layout = util::pad_to_scalar(new_size).ok()?;

        let mut old_block = BlockPtr::from_mem_region(ptr)?;
        if !old_block.verify() {
            panic!("Unable to verify {} at {:p}", old_block.as_ref(), old_block);
        }

        let old_block_size = old_block.size();

        // Shrink allocated block if size is smaller.
        if new_layout.size() < old_block_size {
            if let Some(rem_block) = old_block.shrink(new_layout.size()) {
                self.insert(rem_block);
            }
            return Some(ptr);
        }

        // Just return pointer if size didn't change.
        if new_layout.size() == old_block_size {
            return Some(ptr);
        }

        // Allocate new region to fit size.
        let new_ptr = self._alloc(new_layout)?;

        let copy_size = cmp::min(new_layout.size(), old_block_size);
        intrinsics::volatile_copy_nonoverlapping_memory(new_ptr.as_ptr(), ptr.as_ptr(), copy_size);
        // Add old block back to heap structure.
        self.insert(old_block);
        Some(new_ptr)
    }
}

unsafe impl GlobalAlloc for Collam {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self._alloc(layout) {
            Some(p) => p.cast::<u8>().as_ptr(),
            None => null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if let Some(p) = Unique::new(ptr) {
            self.dealloc_unchecked(p.cast::<c_void>());
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = match self._alloc(layout) {
            Some(p) => p,
            None => return null_mut(),
        };

        // Initialize memory region with 0.
        intrinsics::volatile_set_memory(ptr.as_ptr(), 0, layout.size());
        ptr.cast::<u8>().as_ptr()
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        let ptr = match Unique::new(ptr) {
            Some(p) => p.cast::<c_void>(),
            None => return null_mut(),
        };

        match self._realloc(ptr, new_size) {
            Some(p) => p.cast::<u8>().as_ptr(),
            None => null_mut(),
        }
    }
}

/// Requests memory for the specified size from kernel
/// and returns a `BlockPtr` to the newly created block or `None` if not possible.
fn request_block(min_size: usize) -> Option<BlockPtr> {
    let size = util::pad_to_align(BLOCK_META_SIZE + min_size, *PAGE_SIZE)
        .ok()?
        .size();
    Some(BlockPtr::new(
        util::sbrk(size as isize)?,
        size - BLOCK_META_SIZE,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util;

    #[test]
    fn test_request_block() {
        let block = request_block(256).expect("unable to request block");
        let brk = block.next_potential_block().as_ptr();
        assert_eq!(brk, util::sbrk(0).expect("sbrk(0) failed").as_ptr());
    }

    #[test]
    fn test_request_block_split() {
        let rem_block = request_block(256)
            .expect("unable to request block")
            .shrink(128)
            .expect("unable to split block");
        let brk = rem_block.next_potential_block().as_ptr();
        assert_eq!(brk, util::sbrk(0).expect("sbrk(0) failed").as_ptr());
    }
}
