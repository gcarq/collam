use core::mem;
use core::ptr::Unique;
use mmap::MappedMemoryArena;

use crate::sources::MemorySource;
use libc_print::libc_eprintln;

pub mod heap;
pub mod mmap;

static MUTEX: spin::Mutex<()> = spin::Mutex::new(());

#[repr(C)]
//#[derive(Debug)]
pub struct Bookkeeper {
    head: Unique<MappedMemoryArena>,
    len: usize,
    capacity: usize,
}

impl Bookkeeper {
    /// Initialize list from given ptr and size
    #[must_use]
    pub fn from<T: MemorySource>(source: T) -> Self {
        let head = source.ptr().cast::<MappedMemoryArena>();
        let capacity = source.size() / mem::size_of::<MappedMemoryArena>();
        debug_assert!(capacity > 0);
        // SAFETY: we know we have a valid pointer
        unsafe { *head.as_ptr() = MappedMemoryArena::new() };
        Self {
            head,
            len: 1,
            capacity,
        }
    }

    /// Resolves the arena responsible for the given thread
    /// or creates a new one if none found
    pub unsafe fn get(&mut self, tid: u64) -> Unique<MappedMemoryArena> {
        let lock = MUTEX.lock();
        if let Some(arena) = self.resolve_arena(tid) {
            dprintln!("get() resolved arena for: {}", tid);
            drop(lock);
            return arena;
        }

        // Find unassigned arena
        for mut arena in self.iter() {
            if arena.as_ref().tid.is_none() {
                arena.as_mut().tid = Some(tid);
                dprintln!("get() found unused arena for: {}", tid);
                drop(lock);
                return arena;
            }
        }

        if let Some(mut arena) = self.extend() {
            arena.as_mut().tid = Some(tid);
            dprintln!("get() created new arena for thread: {}", tid);
            drop(lock);
            return arena;
        }

        panic!("FIXME: unable to extend map");
    }

    /// Extends the instance with one `MappedMemoryArena`.
    /// Returns `Err` if capacity has been reached.
    ///
    /// # Safety
    ///
    /// self.head must be a valid pointer
    unsafe fn extend(&mut self) -> Option<Unique<MappedMemoryArena>> {
        if self.len == self.capacity {
            return None;
        }
        self.len += 1;
        let new = self.head.as_ptr().add(self.len - 1);
        *new = MappedMemoryArena::new();
        //println!("extend: {:?}", self);
        Some(Unique::new_unchecked(new))
    }

    /// Resolves the arena responsible for the given thread
    /// TODO: SAFETY
    unsafe fn resolve_arena(&self, tid: u64) -> Option<Unique<MappedMemoryArena>> {
        self.iter().find(|a| a.as_ref().tid == Some(tid))
    }

    #[inline]
    fn iter(&self) -> Iter {
        Iter {
            next: Some(self.head),
            len: self.len,
            index: 0,
        }
    }
}

struct Iter {
    next: Option<Unique<MappedMemoryArena>>,
    len: usize,
    index: usize,
}

impl Iterator for Iter {
    type Item = Unique<MappedMemoryArena>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            return None;
        }
        self.next.map(|arena| {
            unsafe {
                self.next = Some(Unique::new_unchecked(arena.as_ptr().add(1)));
                self.index += 1;
            }
            arena
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::{MappedMemory, MemorySource};

    #[test]
    fn test_keeper_from() {
        let keeper = Bookkeeper::from(unsafe { MappedMemory::new(50) });
        assert_eq!(keeper.capacity, 1);
        assert_eq!(keeper.len, 1);
    }

    #[test]
    fn test_keeper_extend() {
        let mut keeper = Bookkeeper::from(unsafe { MappedMemory::new(100) });
        assert_eq!(keeper.capacity, 2);
        assert_eq!(keeper.len, 1);
        unsafe { assert!(keeper.extend().is_some()) };
        assert_eq!(keeper.len, 2);
        unsafe { assert!(keeper.extend().is_none()) };
        assert_eq!(keeper.len, 2);
    }

    #[test]
    fn test_keeper_iter() {
        let mut keeper = Bookkeeper::from(unsafe { MappedMemory::new(150) });
        unsafe {
            keeper.extend().unwrap();
            keeper.extend().unwrap();
        }
        assert_eq!(keeper.iter().count(), 3);
        assert_eq!(keeper.len, 3);
        assert_eq!(keeper.len, keeper.capacity);
    }
}
