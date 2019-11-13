use core::{ffi::c_void, intrinsics, ptr::NonNull};

use libc_print::libc_eprintln;

use crate::heap::{self, BlockRegion, BLOCK_REGION_META_SIZE};

#[derive(Copy, Clone)]
pub struct IntrusiveList {
    head: Option<NonNull<BlockRegion>>,
    tail: Option<NonNull<BlockRegion>>,
}

unsafe impl core::marker::Send for IntrusiveList {}

impl IntrusiveList {
    pub const fn new() -> Self {
        IntrusiveList {
            head: None,
            tail: None,
        }
    }

    /// Add a block to the list
    pub fn insert(&mut self, to_insert: NonNull<BlockRegion>) {
        unsafe {
            debug_assert_eq!(
                to_insert.as_ref().prev,
                None,
                "block: {} at {:?}",
                to_insert.as_ref(),
                to_insert
            );
            debug_assert_eq!(
                to_insert.as_ref().next,
                None,
                "block: {} at {:?}",
                to_insert.as_ref(),
                to_insert
            );
        }

        // Add initial element
        if self.head.is_none() {
            debug_assert_eq!(self.tail, None);
            self.head = Some(to_insert);
            self.tail = Some(to_insert);
            return;
        }

        debug_assert_ne!(self.head, None);
        debug_assert_ne!(self.tail, None);

        unsafe {
            match self.find_higher_block(to_insert) {
                Err(()) => {
                    eprintln!(
                        "double free detected for ptr {:?}",
                        heap::get_mem_region(to_insert)
                    );
                    return;
                }
                Ok(m) => match m {
                    Some(block) => {
                        self.insert_before(block, to_insert);
                        self.scan_merge(to_insert);
                    }
                    None => self.insert_after(self.tail.unwrap(), to_insert),
                },
            }
        }
    }

    /// Add block to the list before the given element
    unsafe fn insert_before(
        &mut self,
        mut before: NonNull<BlockRegion>,
        mut to_insert: NonNull<BlockRegion>,
    ) {
        // Update links in new block
        to_insert.as_mut().prev = before.as_ref().prev;
        to_insert.as_mut().next = Some(before);

        // Update link for element after new block
        before.as_mut().prev = Some(to_insert);

        // Update link for element before new block
        if let Some(mut prev) = to_insert.as_ref().prev {
            prev.as_mut().next = Some(to_insert);
        }
        self.update_ends(to_insert);
    }

    /// Add block to the list after the given element
    unsafe fn insert_after(
        &mut self,
        mut after: NonNull<BlockRegion>,
        mut to_insert: NonNull<BlockRegion>,
    ) {
        // Update links in new block
        to_insert.as_mut().next = after.as_ref().next;
        to_insert.as_mut().prev = Some(after);

        // Update link for element before new block
        after.as_mut().next = Some(to_insert);

        // Update link for element after new block
        if let Some(mut next) = to_insert.as_ref().next {
            next.as_mut().prev = Some(to_insert);
        }
        self.update_ends(to_insert);
    }

    /// Checks if head or tail should be updated with current block
    #[inline]
    unsafe fn update_ends(&mut self, block: NonNull<BlockRegion>) {
        // Update head if necessary
        if block.as_ref().prev.is_none() {
            self.head = Some(block);
        }

        // Update tail if necessary
        if block.as_ref().next.is_none() {
            self.tail = Some(block);
        }
    }

    /// Takes pointers to two continuous blocks and merges them.
    /// Returns a merged pointer if merge was possible, None otherwise.
    /// NOTE: This function does not modify head or tail.
    unsafe fn maybe_merge_with_next(
        &self,
        mut block: NonNull<BlockRegion>,
    ) -> Option<NonNull<BlockRegion>> {
        let next = block.as_ref().next?;
        if heap::get_next_potential_block(block) != next {
            return None;
        }

        dprintln!("[merge]: {} at {:?}", block.as_ref(), block);
        dprintln!("       & {} at {:?}", next.as_ref(), next);
        // Update related links
        block.as_mut().next = next.as_ref().next;
        if let Some(mut n) = block.as_ref().next {
            n.as_mut().prev = Some(block);
        }
        // Update to final size
        block.as_mut().size += BLOCK_REGION_META_SIZE + next.as_ref().size;

        // Overwrite BlockRegion meta data for old block to detect double free
        intrinsics::volatile_set_memory(next.cast::<c_void>().as_ptr(), 0, BLOCK_REGION_META_SIZE);

        dprintln!("      -> {} at {:?}", block.as_ref(), block);
        return Some(block);
    }

    /// Iterator from the given block forward and merges all blocks possible.
    unsafe fn scan_merge(&mut self, block: NonNull<BlockRegion>) {
        let mut ptr = Some(block);
        while let Some(b) = ptr {
            ptr = self.maybe_merge_with_next(b);
        }
        self.update_ends(block);
    }

    /// Returns first block that has a higher memory address than the given block.
    /// TODO: implement as binary search
    fn find_higher_block(
        &self,
        to_insert: NonNull<BlockRegion>,
    ) -> Result<Option<NonNull<BlockRegion>>, ()> {
        let mut ptr = self.head;
        while let Some(block) = ptr {
            if block == to_insert {
                // block is already in list.
                // One reason for this is double free()
                return Err(());
            } else if block > to_insert {
                return Ok(Some(block));
            }
            ptr = unsafe { block.as_ref().next };
        }
        return Ok(None);
    }

    /// Removes the given element from the list and returns it.
    unsafe fn remove(&mut self, mut elem: NonNull<BlockRegion>) -> NonNull<BlockRegion> {
        // Update head
        if let Some(head) = self.head {
            if elem == head {
                self.head = elem.as_ref().next;
            }
        }
        // Update tail
        if let Some(tail) = self.tail {
            if elem == tail {
                self.tail = elem.as_ref().prev;
            }
        }

        // Update link in previous element
        if let Some(mut prev) = elem.as_ref().prev {
            prev.as_mut().next = elem.as_ref().next;
        }
        // Update link in next element
        if let Some(mut next) = elem.as_ref().next {
            next.as_mut().prev = elem.as_ref().prev;
        }

        // Clear links in current element
        elem.as_mut().next = None;
        elem.as_mut().prev = None;
        return elem;
    }

    /// Prints some debugging information about the heap structure
    pub fn debug(&self) {
        let mut i = 0;
        let mut ptr = self.head;
        while let Some(block) = ptr {
            unsafe {
                dprintln!("[debug]: pos: {}\t{} at\t{:?}", i, block.as_ref(), block);
                block.as_ref().verify(true, true);

                match block.as_ref().prev {
                    Some(prev) => {
                        debug_assert_eq!(prev.as_ref().next.unwrap(), block);
                        // rule out self reference
                        debug_assert_ne!(prev, block);
                    }
                    None => debug_assert_eq!(self.head.unwrap(), block),
                }

                match block.as_ref().next {
                    Some(next) => {
                        debug_assert_eq!(next.as_ref().prev.unwrap(), block);
                        // rule out self reference
                        debug_assert_ne!(next, block);
                    }
                    None => debug_assert_eq!(self.tail.unwrap(), block),
                }

                if let Some(next) = block.as_ref().next {
                    debug_assert!(block < next, "{:?} is not smaller than {:?}", block, next);
                }
                ptr = block.as_ref().next;
                i += 1;
            }
        }
    }

    /// Removes and returns the first suitable block
    pub fn pop(&mut self, size: usize) -> Option<NonNull<BlockRegion>> {
        let mut ptr = self.head;
        while let Some(block) = ptr {
            unsafe {
                if size <= block.as_ref().size {
                    dprintln!(
                        "[libdmalloc.so]: found suitable {} at {:?} for size {}",
                        block.as_ref(),
                        block,
                        size
                    );
                    return Some(self.remove(block));
                }
                ptr = block.as_ref().next;
            }
        }
        None
    }
}
