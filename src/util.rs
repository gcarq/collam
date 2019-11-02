use core::ffi::c_void;

/// Returns a fixed number of bytes that is larger than min_size and
/// a multiple of _SC_PAGESIZE
#[inline]
pub fn alloc_unit(min_size: usize) -> isize {
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };

    for i in 1.. {
        let size = i * page_size as isize;
        if size >= min_size as isize {
            return size;
        }
    }
    panic!("Unable to request {} bytes", min_size);
}

/// Returns a pointer to the current program break
#[inline]
pub fn get_program_break() -> *mut c_void {
    unsafe { libc::sbrk(0) }
}
