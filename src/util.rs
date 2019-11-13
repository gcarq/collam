use core::ffi::c_void;
use core::mem::align_of;
use core::ptr::Unique;

/// Returns a fixed number of bytes that is larger than min_size and
/// a multiple of _SC_PAGESIZE
#[inline]
pub fn alloc_unit(min_size: usize) -> usize {
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;

    let mut size: usize = 2 * page_size;
    loop {
        if size >= min_size {
            return size;
        }
        size *= 2;
    }
}

/// Returns a pointer to the current program break
#[inline]
pub unsafe fn get_program_break() -> Option<Unique<c_void>> {
    sbrk(0)
}

#[inline]
pub fn sbrk(size: isize) -> Option<Unique<c_void>> {
    let ptr = unsafe { libc::sbrk(size) };
    if ptr == -1_isize as *mut c_void {
        return None;
    }
    Unique::new(ptr)
}

/// Aligns passed value to libc::max_align_t
/// FIXME: can overflow if size is slightly less than usize::MAX
#[inline]
pub const fn align_val(val: usize) -> usize {
    let align = align_of::<libc::max_align_t>() - 1;
    (val + align) & !align
}
