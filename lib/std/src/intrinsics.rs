//! Compiler intrinsics. Calls to these are lowered by the MIR builder
//! (src/mir_build.c: lower_intrinsic) or by cgen (src/cgen.c:
//! emit_intrinsic). Adding one means adding a case there.

extern "rcc-intrinsic" {
    /// sizeof(T) in the generated C (never 0: zero-sized types take 1 byte).
    pub fn size_of<T>() -> usize;
    pub fn align_of<T>() -> usize;
    pub fn needs_drop<T: ?Sized>() -> bool;
    /// Run the drop glue of the pointee (elements for slices, the vtable's
    /// drop for trait objects).
    pub fn drop_in_place<T: ?Sized>(p: *mut T);
    /// Mark a value as moved without dropping it.
    pub fn forget<T>(x: T);
    /// Bitwise copy out of `*p` (no move tracking).
    pub fn read<T>(p: *const T) -> T;
    /// Bitwise store to `*p` without dropping the old value.
    pub fn write<T>(p: *mut T, x: T);
    pub fn transmute<A, B>(x: A) -> B;
    pub fn slice_len<T>(s: *const [T]) -> usize;
    pub fn slice_from_raw_parts<T>(p: *const T, len: usize) -> *const [T];
    pub fn discriminant_value<T>(x: *const T) -> isize;
    pub fn ptr_offset<T>(p: *const T, n: isize) -> *const T;
    /// Number of elements between two pointers into the same allocation.
    pub fn ptr_diff<T>(a: *const T, b: *const T) -> isize;
    pub fn copy_nonoverlapping<T>(src: *const T, dst: *mut T, count: usize);
    pub fn copy<T>(src: *const T, dst: *mut T, count: usize);
    pub fn write_bytes<T>(dst: *mut T, val: u8, count: usize);
    pub fn unreachable() -> !;
    pub fn abort() -> !;
    pub fn type_name<T: ?Sized>() -> &'static str;
    pub fn type_id<T: ?Sized>() -> u64;
    pub fn ctpop<T>(x: T) -> u32;
    pub fn ctlz<T>(x: T) -> u32;
    pub fn cttz<T>(x: T) -> u32;
    pub fn bswap<T>(x: T) -> T;
    pub fn rotate_left<T>(x: T, n: u32) -> T;
    pub fn rotate_right<T>(x: T, n: u32) -> T;
    pub fn add_with_overflow<T>(a: T, b: T) -> (T, bool);
    pub fn sub_with_overflow<T>(a: T, b: T) -> (T, bool);
    pub fn mul_with_overflow<T>(a: T, b: T) -> (T, bool);
}
