use core::{fmt, mem};

use core::ffi::c_void;
use libc_print::libc_eprintln;

#[repr(C)]
pub struct BlockRegion {
    pub size: usize,
    pub used: bool,
    next: Option<*mut BlockRegion>,
    prev: Option<*mut BlockRegion>,
}

impl BlockRegion {
    pub fn new(size: usize) -> Self {
        BlockRegion {
            size,
            used: false,
            next: None,
            prev: None,
        }
    }
}

pub const BLOCK_REGION_META_SIZE: usize = mem::size_of::<BlockRegion>();

impl fmt::Display for BlockRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BlockRegion(size={}, prev={:?}, next={:?}, meta_size={})",
            self.size, self.prev, self.next, BLOCK_REGION_META_SIZE,
        )
    }
}

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

    /// Splits the given block in-place to have the exact memory size as specified (excluding metadata).
    /// Returns a newly created block with the remaining size or None if split is not possible.
    pub fn split(&self, block: *mut BlockRegion, size: usize) -> Option<*mut BlockRegion> {
        unsafe { log!("[split]: {} at {:?}", *block, block) }

        // Align pointer of new block
        let new_blk_offset = (BLOCK_REGION_META_SIZE + size + 1).next_power_of_two();
        // Check if its possible to split the block with the requested size
        let new_blk_size = unsafe { (*block).size }
            .checked_sub(new_blk_offset)?
            .checked_sub(BLOCK_REGION_META_SIZE)?;
        if new_blk_size == 0 {
            log!("      -> None");
            return None;
        }

        unsafe {
            assert!(
                new_blk_offset + BLOCK_REGION_META_SIZE < (*block).size,
                "(left={}, right={})",
                new_blk_offset + BLOCK_REGION_META_SIZE,
                (*block).size
            );

            // Update size for old block
            (*block).size = size;

            // Create block with remaining size
            let new_block = block
                .cast::<c_void>()
                .offset(new_blk_offset as isize)
                .cast::<BlockRegion>();
            *new_block = BlockRegion::new(new_blk_size);

            log!("      -> {} at {:?}", *block, block);
            log!("      -> {} at {:?}", *new_block, new_block);

            return Some(new_block);
        };
    }

    /// Removes the given element from the list.
    pub fn remove(&self, elem: *mut BlockRegion) {
        unsafe {
            //assert!((*elem).prev != None || (*elem).next != None);

            // Remove link in previous element
            if let Some(prev) = (*elem).prev {
                (*prev).next = (*elem).next;
            }

            // Remove link in next element
            if let Some(next) = (*elem).next {
                (*next).prev = (*elem).prev;
            }

            (*elem).next = None;
            (*elem).prev = None;
        }
    }

    pub fn debug(&self) {
        for (i, item) in self.into_iter().enumerate() {
            unsafe {
                log!("[debug]: pos: {}\t{} at\t{:?}", i, *item, item);
                //TODO:
                /*if let Some(next) = (*elem).next {
                    assert!(elem < next);
                }*/
            }
        }
    }

    /// Returns the first suitable block found
    pub fn find_block(&self, size: usize) -> Option<*mut BlockRegion> {
        for block in self.into_iter() {
            unsafe {
                if !(*block).used && size <= (*block).size {
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

/// Returns a pointer to the BlockMeta struct from the given memory region raw pointer
#[inline]
pub fn get_block_meta(ptr: *mut c_void) -> *mut BlockRegion {
    unsafe { ptr.cast::<BlockRegion>().offset(-1) }
}

/// Returns a pointer to the assigned memory region for the given block
#[inline]
pub fn get_mem_region(block: *mut BlockRegion) -> *mut c_void {
    unsafe { block.offset(1).cast::<c_void>() }
}
