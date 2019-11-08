use crate::heap::{get_next_block, BlockRegion, BLOCK_REGION_META_SIZE};
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
                Some(block) => self.insert_before(block, to_insert),
                None => self.insert_after(self.tail.unwrap(), to_insert),
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

        match self.maybe_merge(to_insert, before) {
            Some(block) => self.update_ends(block),
            None => self.update_ends(to_insert),
        }
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

        /*let block = self.maybe_merge(after, to_insert);
        self.update_ends(block);*/
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
    unsafe fn maybe_merge(
        &self,
        block1: *mut BlockRegion,
        block2: *mut BlockRegion,
    ) -> Option<*mut BlockRegion> {
        debug_assert!(block1 < block2);
        if get_next_block(block1) != block2 {
            return None;
        }

        eprintln!("[merge]: {} at {:?}", *block1, block1);
        eprintln!("       & {} at {:?}", *block2, block2);
        // Update related links
        debug_assert!(block1 < block2);
        debug_assert_eq!(get_next_block(block1), block2);

        (*block1).next = (*block2).next;
        if let Some(next) = (*block1).next {
            (*next).prev = Some(block1);
        }
        // Update to final size
        (*block1).size += BLOCK_REGION_META_SIZE + (*block2).size;
        eprintln!("      -> {} at {:?}", *block1, block1);
        return Some(block1);
    }

    /// Returns first block that has a higher memory address than the given block.
    fn find_higher_block(&self, to_insert: *mut BlockRegion) -> Option<*mut BlockRegion> {
        let mut ptr = self.head;
        while let Some(block) = ptr {
            if block > to_insert {
                return Some(block);
            }
            ptr = unsafe { (*block).next };
        }
        return None;
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
