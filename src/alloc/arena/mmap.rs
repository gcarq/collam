use libc_print::libc_eprintln;

use crate::alloc::block::BlockPtr;
use crate::alloc::list::IntrusiveList;
use crate::sources::{MappedMemory, MemorySource};
use crate::util;

#[repr(C)]
pub struct MappedMemoryArena {
    pub list: IntrusiveList,
    pub tid: Option<u64>,
    source: MappedMemory,
}

impl MappedMemoryArena {
    #[must_use]
    pub fn new() -> Self {
        // TODO: set sane default
        let source = unsafe { MappedMemory::new(10_131_072) };
        Self {
            list: IntrusiveList::from(&source).expect("unable to initialize list"),
            source,
            tid: None,
        }
    }

    /// Requests and returns a suitable empty `BlockPtr` for the given size.
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
        panic!("FIXME: request() for size: {}, {}", size, util::gettid());
    }

    /// Releases a given `BlockPtr` back to the allocator.
    ///
    /// # Safety
    ///
    /// Function is not thread safe.
    pub unsafe fn release(&mut self, block: BlockPtr) {
        #[cfg(feature = "debug")]
        self.list.debug();

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
        unsafe {
            let mut mem = MappedMemoryArena::new();
            let block = mem.request(256).expect("unable to request block");
            // test that memory region is writable
            intrinsics::volatile_set_memory(block.mem_region().as_ptr(), 42, block.size());
            mem.release(block);
        }
    }

    #[test]
    fn test_request_block_split() {
        unsafe {
            let mut mem = MappedMemoryArena::new();
            let rem_block = mem
                .request(256)
                .expect("unable to request block")
                .shrink(128)
                .expect("unable to split block");
            // test that memory region is writable
            intrinsics::volatile_set_memory(rem_block.mem_region().as_ptr(), 42, rem_block.size());
            mem.release(rem_block);
        }
    }
}
