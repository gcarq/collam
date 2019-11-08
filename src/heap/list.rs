use crate::heap::BlockRegion;
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
    pub fn insert(&mut self, block: *mut BlockRegion) {
        unsafe {
            debug_assert_eq!((*block).prev, None, "block: {} at {:?}", *block, block);
            debug_assert_eq!((*block).next, None, "block: {} at {:?}", *block, block);
        }

        // Add initial element
        if self.head.is_none() {
            debug_assert_eq!(self.tail, None);
            self.head = Some(block);
            self.tail = Some(block);
            return;
        }

        debug_assert_ne!(self.head, None);
        debug_assert_ne!(self.tail, None);

        // TODO: remove unwrap at some point
        unsafe { self.insert_after(self.tail.unwrap(), block) };
    }

    /// Add a block to the list after the given block
    unsafe fn insert_after(&mut self, after: *mut BlockRegion, to_insert: *mut BlockRegion) {
        // Update links in new element
        (*to_insert).next = (*after).next;
        (*to_insert).prev = Some(after);

        // Update link in existing element
        (*after).next = Some(to_insert);

        // Update tail if necessary
        if (*to_insert).next.is_none() {
            self.tail = Some(to_insert);
        }
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
                (*block).verify(true);

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

                /*if let Some(next) = (*item).next {
                    debug_assert!(item < next, "{:?} is not smaller than {:?}", item, next);
                }*/
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
