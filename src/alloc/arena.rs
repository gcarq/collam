use libc_print::libc_eprintln;

use crate::alloc::block::BlockPtr;
use crate::alloc::list::IntrusiveList;
use crate::sources::{HeapSegment, MemorySource};

pub struct Arena {
    pub list: IntrusiveList,
    source: HeapSegment,
}

impl Arena {
    pub const fn new() -> Self {
        Self {
            list: IntrusiveList::new(),
            source: HeapSegment,
        }
    }

    /// Requests and returns a suitable empty `BlockPtr` for the given size.
    /// This can be either a reused empty block or a new one requested from kernel.
    ///
    /// # Safety
    ///
    /// Function is not thread safe.
    pub unsafe fn request(&mut self, size: usize) -> Option<BlockPtr> {
        if let Some(block) = self.list.pop(size) {
            dprintln!("[pop]: {} at {:p}", block.as_ref(), block);
            return Some(block);
        }
        self.source.request(size)
    }

    /// Releases a given `BlockPtr` back to the allocator or kernel.
    ///
    /// # Safety
    ///
    /// Function is not thread safe.
    pub unsafe fn release(&mut self, block: BlockPtr) {
        #[cfg(feature = "debug")]
        self.list.debug();

        if self.source.release(block) {
            return;
        }

        dprintln!("[insert]: {} at {:p}", block.as_ref(), block);
        if self.list.insert(block).is_err() {
            eprintln!("double free detected for ptr {:?}", block.mem_region());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::ffi::c_void;
    use libc::sbrk;

    #[test]
    fn test_request_block() {
        unsafe {
            let mut heap = Arena::new();
            let block = heap.request(256).expect("unable to request block");
            let brk = block.next_potential_block().as_ptr();
            assert_eq!(brk.cast::<c_void>(), sbrk(0));
        }
    }

    #[test]
    fn test_request_block_split() {
        unsafe {
            let mut heap = Arena::new();
            let rem_block = heap
                .request(256)
                .expect("unable to request block")
                .shrink(128)
                .expect("unable to split block");
            let brk = rem_block.next_potential_block().as_ptr();
            assert_eq!(brk.cast::<c_void>(), sbrk(0));
        }
    }
}
