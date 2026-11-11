//! Allocation on top of malloc (rt/rt.c).

extern "C" {
    fn rcc_alloc(size: usize, align: usize) -> *mut u8;
    fn rcc_realloc(p: *mut u8, new_size: usize) -> *mut u8;
    fn rcc_dealloc(p: *mut u8);
}

/// Allocate `size` bytes (aborts on failure; never returns null).
pub unsafe fn alloc_bytes(size: usize, align: usize) -> *mut u8 {
    rcc_alloc(size, align)
}
pub unsafe fn realloc_bytes(p: *mut u8, new_size: usize) -> *mut u8 {
    rcc_realloc(p, new_size)
}
/// Free a block from alloc_bytes/realloc_bytes (null is allowed).
pub unsafe fn dealloc_bytes(p: *mut u8) {
    rcc_dealloc(p)
}

/// Allocate uninitialized storage for `n` values of T; null when n == 0.
pub unsafe fn alloc_array<T>(n: usize) -> *mut T {
    if n == 0 {
        return 0usize as *mut T;
    }
    rcc_alloc(n * crate::intrinsics::size_of::<T>(), crate::intrinsics::align_of::<T>()) as *mut T
}
pub unsafe fn realloc_array<T>(p: *mut T, n: usize) -> *mut T {
    if (p as usize) == 0 {
        return alloc_array::<T>(n);
    }
    rcc_realloc(p as *mut u8, n * crate::intrinsics::size_of::<T>()) as *mut T
}
pub unsafe fn dealloc_array<T>(p: *mut T) {
    rcc_dealloc(p as *mut u8)
}
