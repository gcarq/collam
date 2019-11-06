use crate::heap::BlockRegion;
use libc_print::libc_eprintln;

#[derive(Copy, Clone)]
pub struct IntrusiveList {
    head: Option<*mut BlockRegion>,
    tail: Option<*mut BlockRegion>,
}

impl IntrusiveList {
    pub const fn new() -> Self {
        IntrusiveList {
            head: None,
            tail: None,
        }
    }

    pub fn insert(&mut self, elem: *mut BlockRegion) {
        unsafe {
            assert_eq!((*elem).prev, None);
            assert_eq!((*elem).next, None);
        }

        // Add initial element
        if self.head.is_none() {
            assert_eq!(self.tail, None);
            self.head = Some(elem);
            self.tail = Some(elem);
            return;
        }

        assert_ne!(self.head, None);
        assert_ne!(self.tail, None);

        // TODO: remove unwrap at some point
        self.insert_after(self.tail.unwrap(), elem);
    }

    fn insert_after(&mut self, after: *mut BlockRegion, to_insert: *mut BlockRegion) {
        unsafe {
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
    }

    /// Removes the given element from the list.
    pub fn remove(&mut self, elem: *mut BlockRegion) {
        unsafe {
            // Update link in previous element
            if let Some(prev) = (*elem).prev {
                (*prev).next = (*elem).next;
            }

            // Update link in next element
            if let Some(next) = (*elem).next {
                (*next).prev = (*elem).prev;
            }

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

            // Clear links in current element
            (*elem).next = None;
            (*elem).prev = None;
        }
    }

    pub fn debug(&self) {
        for (i, item) in self.into_iter().enumerate() {
            unsafe {
                (*item).verify();
                log!("[debug]: pos: {}\t{} at\t{:?}", i, *item, item);
                //TODO:
                /*if let Some(next) = (*elem).next {
                    assert!(elem < next);
                }*/
            }
        }
    }

    /// Returns the first suitable block found
    pub fn find(&self, size: usize) -> Option<*mut BlockRegion> {
        for block in self.into_iter() {
            unsafe {
                if size <= (*block).size {
                    log!(
                        "[libdmalloc.so]: found suitable {} at {:?} for size {}",
                        *block,
                        block,
                        size
                    );
                    return Some(block);
                }
            }
        }
        None
    }
}

impl IntoIterator for IntrusiveList {
    type Item = *mut BlockRegion;
    type IntoIter = ListIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        ListIntoIter { node: self.head }
    }
}

/// Iterator for simply traversing the LinkedList
pub struct ListIntoIter {
    node: Option<*mut BlockRegion>,
}

impl Iterator for ListIntoIter {
    type Item = *mut BlockRegion;

    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.node;
        if let Some(node) = cur {
            self.node = unsafe { (*node).next };
        }
        return cur;
    }
}
