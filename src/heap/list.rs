use crate::heap::{get_mem_region, get_next_block, BlockRegion, BLOCK_REGION_META_SIZE};
use core::ffi::c_void;
use core::intrinsics;
use libc_print::libc_eprintln;

#[derive(Copy, Clone)]
pub struct IntrusiveList {
    head: Option<*mut BlockRegion>,
    tail: Option<*mut BlockRegion>,
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
    pub fn insert(&mut self, to_insert: *mut BlockRegion) {
        unsafe {
            debug_assert_eq!(
                (*to_insert).prev,
                None,
                "block: {} at {:?}",
                *to_insert,
                to_insert
            );
            debug_assert_eq!(
                (*to_insert).next,
                None,
                "block: {} at {:?}",
                *to_insert,
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
                        get_mem_region(to_insert)
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
    unsafe fn insert_before(&mut self, before: *mut BlockRegion, to_insert: *mut BlockRegion) {
        // Update links in new block
        (*to_insert).prev = (*before).prev;
        (*to_insert).next = Some(before);

        // Update link for element after new block
        (*before).prev = Some(to_insert);

        // Update link for element before new block
        if let Some(prev) = (*to_insert).prev {
            (*prev).next = Some(to_insert);
        }
        self.update_ends(to_insert);
    }

    /// Add block to the list after the given element
    unsafe fn insert_after(&mut self, after: *mut BlockRegion, to_insert: *mut BlockRegion) {
        // Update links in new block
        (*to_insert).next = (*after).next;
        (*to_insert).prev = Some(after);

        // Update link for element before new block
        (*after).next = Some(to_insert);

        // Update link for element after new block
        if let Some(next) = (*to_insert).next {
            (*next).prev = Some(to_insert);
        }
        self.update_ends(to_insert);
    }

    /// Checks if head or tail should be updated with current block
    #[inline]
    unsafe fn update_ends(&mut self, block: *mut BlockRegion) {
        // Update head if necessary
        if (*block).prev.is_none() {
            self.head = Some(block);
        }

        // Update tail if necessary
        if (*block).next.is_none() {
            self.tail = Some(block);
        }
    }

    /// Takes pointers to two continuous blocks and merges them.
    /// Returns a merged pointer if merge was possible, None otherwise.
    /// NOTE: This function does not modify head or tail.
    unsafe fn maybe_merge_with_next(&self, block: *mut BlockRegion) -> Option<*mut BlockRegion> {
        let next = (*block).next?;
        if get_next_block(block) != next {
            return None;
        }

        dprintln!("[merge]: {} at {:?}", *block, block);
        dprintln!("       & {} at {:?}", *next, next);
        // Update related links
        (*block).next = (*next).next;
        if let Some(n) = (*block).next {
            (*n).prev = Some(block);
        }
        // Update to final size
        (*block).size += BLOCK_REGION_META_SIZE + (*next).size;

        // Overwrite BlockRegion meta data for old block to detect double free
        intrinsics::volatile_set_memory(next.cast::<c_void>(), 0, BLOCK_REGION_META_SIZE);

        dprintln!("      -> {} at {:?}", *block, block);
        return Some(block);
    }

    /// Iterator from the given block forward and merges all blocks possible.
    unsafe fn scan_merge(&mut self, block: *mut BlockRegion) {
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
        to_insert: *mut BlockRegion,
    ) -> Result<Option<*mut BlockRegion>, ()> {
        let mut ptr = self.head;
        while let Some(block) = ptr {
            if block == to_insert {
                // block is already in list.
                // One reason for this is double free()
                return Err(());
            } else if block > to_insert {
                return Ok(Some(block));
            }
            ptr = unsafe { (*block).next };
        }
        return Ok(None);
    }

    /// Removes the given element from the list and returns it.
    unsafe fn remove(&mut self, elem: *mut BlockRegion) -> *mut BlockRegion {
        // Update head
        if let Some(head) = self.head {
            if elem == head {
                self.head = (*elem).next;
            }
        }
        // Update tail
        if let Some(tail) = self.tail {
            if elem == tail {
                self.tail = (*elem).prev;
            }
        }

        // Update link in previous element
        if let Some(prev) = (*elem).prev {
            (*prev).next = (*elem).next;
        }
        // Update link in next element
        if let Some(next) = (*elem).next {
            (*next).prev = (*elem).prev;
        }

        // Clear links in current element
        (*elem).next = None;
        (*elem).prev = None;
        return elem;
    }

    /// Prints some debugging information about the heap structure
    pub fn debug(&self) {
        let mut i = 0;
        let mut ptr = self.head;
        while let Some(block) = ptr {
            unsafe {
                dprintln!("[debug]: pos: {}\t{} at\t{:?}", i, *block, block);
                (*block).verify(true, true);

                match (*block).prev {
                    Some(prev) => {
                        debug_assert_eq!((*prev).next.unwrap(), block);
                        // rule out self reference
                        debug_assert_ne!(prev, block);
                    }
                    None => debug_assert_eq!(self.head.unwrap(), block),
                }

                match (*block).next {
                    Some(next) => {
                        debug_assert_eq!((*next).prev.unwrap(), block);
                        // rule out self reference
                        debug_assert_ne!(next, block);
                    }
                    None => debug_assert_eq!(self.tail.unwrap(), block),
                }

                if let Some(next) = (*block).next {
                    debug_assert!(block < next, "{:?} is not smaller than {:?}", block, next);
                }
                ptr = (*block).next;
                i += 1;
            }
        }
    }

    /// Removes and returns the first suitable block
    pub fn pop(&mut self, size: usize) -> Option<*mut BlockRegion> {
        let mut ptr = self.head;
        while let Some(block) = ptr {
            unsafe {
                if size <= (*block).size {
                    dprintln!(
                        "[libdmalloc.so]: found suitable {} at {:?} for size {}",
                        *block,
                        block,
                        size
                    );
                    return Some(self.remove(block));
                }
                ptr = (*block).next;
            }
        }
        None
    }
}
