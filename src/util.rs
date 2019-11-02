/// Returns a fixed number of bytes that is a multiple of the memory page size
#[inline]
pub fn alloc_unit() -> usize {
    return unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize * 3
}