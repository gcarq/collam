use core::ffi::c_void;

/// Returns a fixed number of bytes that is larger than min_size and
/// a multiple of _SC_PAGESIZE
#[inline]
pub fn alloc_unit(min_size: usize) -> usize {
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;

    let mut size: usize = 4 * page_size;
    loop {
        if size >= min_size {
            return size;
        }
        size *= 2;
    }
}

/// Returns a pointer to the current program break
#[inline]
pub unsafe fn get_program_break() -> *mut c_void {
    libc::sbrk(0)
}

/// Align passed value to multiple of 16
/// TODO: can overflow if size is slightly less than usize::MAX
#[inline]
pub const fn align_next_mul_16(val: usize) -> usize {
    (val + 15) & !15usize
}
