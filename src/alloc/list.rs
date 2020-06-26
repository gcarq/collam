use libc_print::libc_eprintln;

use crate::alloc::block::{BlockPtr, BLOCK_META_SIZE, BLOCK_SPLIT_MIN_SIZE};
use crate::sources::MemorySource;

#[repr(C)]
pub struct IntrusiveList {
    pub head: Option<BlockPtr>,
    pub tail: Option<BlockPtr>,
}

impl IntrusiveList {
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    /// Initialize list from given ptr and size
    pub fn from<T: MemorySource>(source: &T) -> Result<Self, ()> {
        debug_assert!(source.size() > BLOCK_META_SIZE);
        let mut instance = Self::new();
        instance.insert(BlockPtr::new(source.ptr(), source.size() - BLOCK_META_SIZE))?;
        Ok(instance)
    }

    /// Inserts a `BlockPtr` to the existing list and
    /// returns `Err` on detected double-free.
    pub fn insert(&mut self, mut to_insert: BlockPtr) -> Result<(), ()> {
        // Reset pointer locations since they were part as user allocatable data
        to_insert.as_mut().unlink();

        // Add initial element
        if self.head.is_none() {
            debug_assert!(self.tail.is_none());
            self.head = Some(to_insert);
            self.tail = Some(to_insert);
            return Ok(());
        }

        debug_assert!(self.head.is_some());
        debug_assert!(self.tail.is_some());

        match self.find_higher_block(to_insert)? {
            Some(block) => IntrusiveList::insert_before(block, to_insert),
            None => IntrusiveList::insert_after(self.tail.unwrap(), to_insert),
        }
        let inserted = IntrusiveList::maybe_merge_adjacent(to_insert);
        self.update_ends(inserted);
        Ok(())
    }

    /// Removes and returns the first suitable `BlockPtr`.
    pub fn pop(&mut self, size: usize) -> Option<BlockPtr> {
        for block in self.iter() {
            if size == block.size() {
                dprintln!(
                    "[libcollam.so]: found perfect {} at {:p} for size {}",
                    block.as_ref(),
                    block,
                    size
                );
                return Some(self.remove(block));
            }
            if size + BLOCK_SPLIT_MIN_SIZE <= block.size() {
                dprintln!(
                    "[libcollam.so]: found suitable {} at {:p} for size {}",
                    block.as_ref(),
                    block,
                    size
                );
                return Some(self.remove(block));
            }
        }
        None
    }

    /// Prints some debugging information about the heap structure.
    #[cfg(feature = "debug")]
    pub fn debug(&self) {
        dprintln!("[debug]: === list debug start ===");
        for (i, block) in self.iter().enumerate() {
            dprintln!("[debug]: pos: {}\t{} at\t{:p}", i, block.as_ref(), block);
            if !block.as_ref().verify() {
                panic!("Unable to verify: {} at\t{:p}", block.as_ref(), block);
            }

            match block.as_ref().prev {
                Some(prev) => {
                    debug_assert_eq!(prev.as_ref().next.unwrap().as_ptr(), block.as_ptr());
                    // rule out self reference
                    debug_assert_ne!(prev.as_ptr(), block.as_ptr());
                }
                None => debug_assert_eq!(self.head.unwrap().as_ptr(), block.as_ptr()),
            }

            match block.as_ref().next {
                Some(next) => {
                    debug_assert_eq!(next.as_ref().prev.unwrap().as_ptr(), block.as_ptr());
                    // rule out self reference
                    debug_assert_ne!(next.as_ptr(), block.as_ptr());
                }
                None => debug_assert_eq!(self.tail.unwrap().as_ptr(), block.as_ptr()),
            }

            if let Some(next) = block.as_ref().next {
                debug_assert!(
                    block.as_ptr() < next.as_ptr(),
                    "{:p} is not smaller than {:p}",
                    block,
                    next
                );
            }
        }
        dprintln!("[debug]: === list debug end ===");
    }

    /// Adds a `BlockPtr` to the list before the given anchor.
    fn insert_before(mut anchor: BlockPtr, mut to_insert: BlockPtr) {
        // Update links in new block
        to_insert.as_mut().prev = anchor.as_ref().prev;
        to_insert.as_mut().next = Some(anchor);

        // Update link for element after new block
        anchor.as_mut().prev = Some(to_insert);

        // Update link for element before new block
        if let Some(mut prev) = to_insert.as_ref().prev {
            prev.as_mut().next = Some(to_insert);
        }
    }

    /// Adds a `BlockPtr` to the list after the given anchor.
    fn insert_after(mut anchor: BlockPtr, mut to_insert: BlockPtr) {
        // Update links in new block
        to_insert.as_mut().next = anchor.as_ref().next;
        to_insert.as_mut().prev = Some(anchor);

        // Update link for element before new block
        anchor.as_mut().next = Some(to_insert);

        // Update link for element after new block
        if let Some(mut next) = to_insert.as_ref().next {
            next.as_mut().prev = Some(to_insert);
        }
    }

    /// Checks if head or tail should be updated with the given `BlockPtr`.
    fn update_ends(&mut self, block: BlockPtr) {
        // Update head if necessary
        if block.as_ref().prev.is_none() {
            self.head = Some(block);
        }

        // Update tail if necessary
        if block.as_ref().next.is_none() {
            self.tail = Some(block);
        }
    }

    /// Takes a `BlockPtr` and tries to merge adjacent blocks if possible.
    /// Always returns a `BlockPtr`.
    fn maybe_merge_adjacent(block: BlockPtr) -> BlockPtr {
        let block = match block.as_ref().prev {
            Some(prev) => prev.maybe_merge_next().unwrap_or(block),
            None => block,
        };
        block.maybe_merge_next().unwrap_or(block)
    }

    /// Returns first `BlockPtr` that has a higher memory address than the given `BlockPtr`
    /// or `None` if no block exists at a higher memory address.
    /// Returns `Err` if given `BlockPtr` is already in list.
    /// TODO: implement with better algorithm
    fn find_higher_block(&self, to_insert: BlockPtr) -> Result<Option<BlockPtr>, ()> {
        for block in self.iter() {
            if block.as_ptr() > to_insert.as_ptr() {
                return Ok(Some(block));
            }
            if block == to_insert {
                // block is already in list.
                // One reason for this is double free()
                return Err(());
            }
        }
        Ok(None)
    }

    /// Removes the given `BlockPtr` from list and returns it.
    fn remove(&mut self, mut elem: BlockPtr) -> BlockPtr {
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
        elem.as_mut().unlink();
        elem
    }

    #[inline]
    pub fn iter(&self) -> Iter {
        Iter { next: self.head }
    }
}

pub struct Iter {
    next: Option<BlockPtr>,
}

impl Iterator for Iter {
    type Item = BlockPtr;
    fn next(&mut self) -> Option<Self::Item> {
        self.next.map(|node| {
            self.next = node.as_ref().next;
            node
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc::arena::heap::HeapArena;
    use crate::alloc::block::BLOCK_META_SIZE;
    use crate::sources::{HeapSegment, MemorySource};

    #[test]
    fn test_list_new() {
        let list = IntrusiveList::new();
        assert_eq!(list.head, None);
        assert_eq!(list.tail, None);
    }

    #[test]
    fn test_list_from() {
        let source = unsafe { HeapSegment::new(4096) };
        let list = IntrusiveList::from(&source).unwrap();
        assert!(list.head.is_some());
        assert!(list.tail.is_some());
        assert_eq!(list.head, list.tail);
    }

    #[test]
    fn test_insert_after_not_adjacent() {
        let mut heap = HeapArena::new();
        let mut block = unsafe { heap.request(256).expect("unable to request block") };
        // Block2 imitates a used block. So it will not be added to list
        let mut block2 = block.shrink(64).expect("unable to split block");
        let block3 = block2.shrink(64).expect("unable to split block");

        // Insert block1
        heap.list.insert(block).expect("unable to insert");
        assert_eq!(heap.list.head, Some(block));
        // Tail might be another block that is split of to match requested size
        assert!(heap.list.tail.is_some());
        assert_eq!(block.as_ref().prev, None);

        // Insert block3
        heap.list.insert(block3).expect("unable to insert");
        assert_eq!(heap.list.head, Some(block));
        assert_eq!(heap.list.tail, Some(block3));
        assert_eq!(block.as_ref().next, Some(block3));
        assert_eq!(block.as_ref().prev, None);
        assert_eq!(block3.as_ref().next, None);
        assert_eq!(block3.as_ref().prev, Some(block));
    }

    #[test]
    fn test_insert_before_not_adjacent() {
        let mut heap = HeapArena::new();
        let mut block = unsafe { heap.request(256).expect("unable to request block") };
        // Block2 imitates a used block. So it will not be added to list
        let mut block2 = block.shrink(64).expect("unable to split block");
        let block3 = block2.shrink(64).expect("unable to split block");

        // Insert block3
        heap.list.insert(block3).expect("unable to insert");
        assert_eq!(heap.list.head, Some(block3));
        assert_eq!(heap.list.tail, Some(block3));
        assert_eq!(block3.as_ref().next, None);
        assert_eq!(block3.as_ref().prev, None);

        // Insert block1
        heap.list.insert(block).expect("unable to insert");
        assert_eq!(heap.list.head, Some(block));
        assert_eq!(heap.list.tail, Some(block3));
        assert_eq!(block.as_ref().next, Some(block3));
        assert_eq!(block.as_ref().prev, None);
        assert_eq!(block3.as_ref().next, None);
        assert_eq!(block3.as_ref().prev, Some(block));
    }

    #[test]
    fn test_insert_merge() {
        let mut heap = HeapArena::new();
        let mut block = unsafe { heap.request(256).expect("unable to request block") };
        let mut block2 = block.shrink(64).expect("unable to split block");
        let block3 = block2.shrink(64).expect("unable to split block");

        // Insert block1
        heap.list.insert(block).expect("unable to insert");
        assert_eq!(heap.list.head, Some(block));
        // Tail might be another block that is split of to match requested size
        assert!(heap.list.tail.is_some());
        assert_eq!(block.as_ref().prev, None);
        assert_eq!(block.size(), 64);

        // Insert block2
        heap.list.insert(block2).expect("unable to insert");
        assert_eq!(heap.list.head, Some(block));
        assert!(heap.list.tail.is_some());
        assert_eq!(block.as_ref().prev, None);
        assert_eq!(block.size(), 64 + BLOCK_META_SIZE + 64);

        // Insert block3
        heap.list.insert(block3).expect("unable to insert");
        assert_eq!(heap.list.head, Some(block));
        assert_eq!(heap.list.tail, Some(block));
        assert_eq!(block.as_ref().next, None);
        assert_eq!(block.as_ref().prev, None);
        assert!(block.size() > 64 + BLOCK_META_SIZE + 64 + BLOCK_META_SIZE);
    }

    #[test]
    fn test_pop_exact_size() {
        let mut heap = HeapArena::new();
        let mut block = unsafe { heap.request(512).expect("unable to request block") };
        // Block2 imitates a used block. So it will not be added to list
        let mut block2 = block.shrink(64).expect("unable to split block");
        let block3 = block2.shrink(64).expect("unable to split block");

        // Insert block1
        heap.list.insert(block).expect("unable to insert");
        // Insert block3
        heap.list.insert(block3).expect("unable to insert");

        let result = heap.list.pop(64).expect("got no block");
        assert_eq!(result, block);
        assert_eq!(result.as_ref().next, None);
        assert_eq!(result.as_ref().prev, None);
        assert_eq!(result.size(), 64);
    }

    #[test]
    fn test_pop_smaller_size() {
        let mut heap = HeapArena::new();
        let mut block = unsafe { heap.request(512).expect("unable to request block") };
        // Block2 imitates a used block. So it will not be added to list
        let mut block2 = block.shrink(64).expect("unable to split block");
        let block3 = block2.shrink(64).expect("unable to split block");

        // Insert block1
        heap.list.insert(block).expect("unable to insert");
        // Insert block3
        heap.list.insert(block3).expect("unable to insert");

        let result = heap.list.pop(16).expect("got no block");
        assert_eq!(result, block);
        assert_eq!(result.as_ref().next, None);
        assert_eq!(result.as_ref().prev, None);
        assert_eq!(result.size(), 64);
    }

    #[test]
    fn test_iter() {
        let mut heap = HeapArena::new();
        let mut block = unsafe { heap.request(256).expect("unable to request block") };
        let mut block2 = block.shrink(64).expect("unable to split block");
        let block3 = block2.shrink(64).expect("unable to split block");

        // Insert block1
        heap.list.insert(block).expect("unable to insert");
        // Insert block3
        heap.list.insert(block3).expect("unable to insert");

        let mut iter = heap.list.iter();
        assert_eq!(iter.next().unwrap(), block);
        assert_eq!(iter.next().unwrap(), block3);
        assert!(iter.next().is_none());
    }

    #[cfg(feature = "debug")]
    #[test]
    fn test_debug() {
        let mut heap = HeapArena::new();
        let mut block = unsafe { heap.request(256).expect("unable to request block") };
        // Block2 imitates a used block. So it will not be added to list
        let mut block2 = block.shrink(64).expect("unable to split block");
        let block3 = block2.shrink(64).expect("unable to split block");

        // Insert block1
        heap.list.insert(block).expect("unable to insert");
        // Insert block3
        heap.list.insert(block3).expect("unable to insert");
        heap.list.debug();
    }
}
