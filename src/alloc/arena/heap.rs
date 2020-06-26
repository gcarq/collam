use libc_print::libc_eprintln;

use crate::alloc::block::BlockPtr;
use crate::alloc::list::IntrusiveList;
use crate::sources::{HeapSegment, MemorySource};

#[repr(C)]
pub struct HeapArena {
    pub list: IntrusiveList,
    source: HeapSegment,
}

impl HeapArena {
    #[must_use]
    pub fn new() -> Self {
        let source = unsafe { HeapSegment::new(131_072) };
        Self {
            list: IntrusiveList::from(&source).expect("unable to initialize list"),
            source,
        }
    }

    /// Requests and returns a suitable empty `BlockPtr` for the given size.
    /// This can be either a reused empty block or a new one requested from kernel.
    ///
    /// # Safety
    ///
    /// Function is not thread safe.
    pub unsafe fn request(&mut self, size: usize) -> Option<BlockPtr> {
        if let Some(mut block) = self.list.pop(size) {
            block.shrink(size).and_then(|b| self.list.insert(b).ok());
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
    use core::intrinsics;

    #[test]
    fn test_request_block() {
        let mut mem = HeapArena::new();
        unsafe {
            let block = mem.request(256).expect("unable to request block");
            // test that memory region is writable
            intrinsics::volatile_set_memory(block.mem_region().as_ptr(), 42, block.size());
            let next = block.next_potential_block().as_ptr();
            assert!(!next.is_null());
            mem.release(block);
        }
    }

    #[test]
    fn test_request_block_split() {
        let mut mem = HeapArena::new();
        unsafe {
            let rem_block = mem
                .request(256)
                .expect("unable to request block")
                .shrink(128)
                .expect("unable to split block");
            // test that memory region is writable
            intrinsics::volatile_set_memory(rem_block.mem_region().as_ptr(), 42, rem_block.size());
            let next = rem_block.next_potential_block().as_ptr();
            assert!(!next.is_null());
            mem.release(rem_block);
        }
    }
}
