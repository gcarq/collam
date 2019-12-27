use core::intrinsics::unlikely;

use libc_print::libc_eprintln;

use crate::alloc::block::{BlockPtr, BLOCK_META_SIZE};
use crate::alloc::list::IntrusiveList;
use crate::util;

lazy_static! {
    static ref PAGE_SIZE: usize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
}

pub struct Heap {
    pub list: IntrusiveList,
}

impl Heap {
    pub const fn new() -> Self {
        Heap {
            list: IntrusiveList::new(),
        }
    }

    /// Requests and returns a suitable empty `BlockPtr` for the given size.
    /// This can be either a reused empty block or a new one requested from kernel.
    pub unsafe fn request(&mut self, size: usize) -> Option<BlockPtr> {
        if let Some(block) = self.list.pop(size) {
            dprintln!("[pop]: {} at {:p}", block.as_ref(), block);
            return Some(block);
        }
        self.request_from_kernel(size)
    }

    /// Releases a given `BlockPtr` back to the allocator or kernel.
    pub unsafe fn release(&mut self, block: BlockPtr) {
        #[cfg(feature = "debug")]
        self.list.debug();

        let brk = util::sbrk(0).expect("sbrk(0) failed!").as_ptr();
        if block.next_potential_block().as_ptr() == brk {
            self.release_to_kernel(block);
            return;
        }

        dprintln!("[insert]: {} at {:p}", block.as_ref(), block);
        if unlikely(self.list.insert(block).is_err()) {
            eprintln!("double free detected for ptr {:?}", block.mem_region());
        }
    }

    /// Requests memory for the specified size from kernel by increasing the break
    /// and returns a `BlockPtr` to the newly created block or `None` if not possible.
    /// Marked as unsafe because it is not thread safe.
    unsafe fn request_from_kernel(&self, min_size: usize) -> Option<BlockPtr> {
        let size = util::pad_to_align(BLOCK_META_SIZE + min_size, *PAGE_SIZE)
            .ok()?
            .size();
        Some(BlockPtr::new(
            util::sbrk(size as isize)?,
            size - BLOCK_META_SIZE,
        ))
    }

    /// Releases given `BlockPtr` back to the kernel by decreasing the break.
    /// Marked as unsafe because it is not thread safe.
    unsafe fn release_to_kernel(&mut self, block: BlockPtr) {
        let offset = block.block_size() as isize;
        dprintln!(
            "[insert]: freeing {} bytes from process (break={:?})",
            offset,
            util::sbrk(0).expect("sbrk(0) failed!").as_ptr()
        );
        // TODO: remove expect
        util::sbrk(-offset).expect("sbrk failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util;

    #[test]
    fn test_request_block() {
        unsafe {
            let heap = Heap::new();
            let block = heap
                .request_from_kernel(256)
                .expect("unable to request block");
            let brk = block.next_potential_block().as_ptr();
            assert_eq!(brk, util::sbrk(0).expect("sbrk(0) failed").as_ptr());
        }
    }

    #[test]
    fn test_request_block_split() {
        unsafe {
            let heap = Heap::new();
            let rem_block = heap
                .request_from_kernel(256)
                .expect("unable to request block")
                .shrink(128)
                .expect("unable to split block");
            let brk = rem_block.next_potential_block().as_ptr();
            assert_eq!(brk, util::sbrk(0).expect("sbrk(0) failed").as_ptr());
        }
    }
}
