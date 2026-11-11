use crate::intrinsics;

pub const fn null<T>() -> *const T {
    0usize as *const T
}
pub const fn null_mut<T>() -> *mut T {
    0usize as *mut T
}
pub unsafe fn read<T>(src: *const T) -> T {
    intrinsics::read(src)
}
pub unsafe fn write<T>(dst: *mut T, src: T) {
    intrinsics::write(dst, src)
}
pub unsafe fn drop_in_place<T: ?Sized>(p: *mut T) {
    intrinsics::drop_in_place(p)
}
pub unsafe fn copy_nonoverlapping<T>(src: *const T, dst: *mut T, count: usize) {
    intrinsics::copy_nonoverlapping(src, dst, count)
}
pub unsafe fn copy<T>(src: *const T, dst: *mut T, count: usize) {
    intrinsics::copy(src, dst, count)
}
pub unsafe fn write_bytes<T>(dst: *mut T, val: u8, count: usize) {
    intrinsics::write_bytes(dst, val, count)
}
pub unsafe fn swap<T>(a: *mut T, b: *mut T) {
    let tmp = intrinsics::read(a as *const T);
    intrinsics::copy(b as *const T, a, 1);
    intrinsics::write(b, tmp);
}
pub unsafe fn replace<T>(dst: *mut T, src: T) -> T {
    let old = intrinsics::read(dst as *const T);
    intrinsics::write(dst, src);
    old
}
/// Pointer identity (addresses only; metadata of fat pointers is ignored).
pub fn eq<T: ?Sized>(a: *const T, b: *const T) -> bool {
    (a as *const u8 as usize) == (b as *const u8 as usize)
}
pub fn addr_eq<T: ?Sized, U: ?Sized>(a: *const T, b: *const U) -> bool {
    (a as *const u8 as usize) == (b as *const u8 as usize)
}
pub fn slice_from_raw_parts<T>(data: *const T, len: usize) -> *const [T] {
    unsafe { intrinsics::slice_from_raw_parts(data, len) }
}
pub fn slice_from_raw_parts_mut<T>(data: *mut T, len: usize) -> *mut [T] {
    unsafe { intrinsics::slice_from_raw_parts(data as *const T, len) as *mut [T] }
}

impl<T> *const T {
    pub fn is_null(self) -> bool {
        self as usize == 0
    }
    pub unsafe fn offset(self, count: isize) -> *const T {
        intrinsics::ptr_offset(self, count)
    }
    pub unsafe fn add(self, count: usize) -> *const T {
        intrinsics::ptr_offset(self, count as isize)
    }
    pub unsafe fn sub(self, count: usize) -> *const T {
        intrinsics::ptr_offset(self, 0 - (count as isize))
    }
    pub fn wrapping_add(self, count: usize) -> *const T {
        unsafe { intrinsics::ptr_offset(self, count as isize) }
    }
    pub unsafe fn read(self) -> T {
        intrinsics::read(self)
    }
    pub unsafe fn offset_from(self, origin: *const T) -> isize {
        intrinsics::ptr_diff(self as *const T, origin)
    }
    pub fn cast<U>(self) -> *const U {
        self as *const U
    }
    pub fn cast_mut(self) -> *mut T {
        self as *mut T
    }
    pub fn addr(self) -> usize {
        self as usize
    }
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        if self.is_null() { None } else { Some(&*self) }
    }
}

impl<T> *mut T {
    pub fn is_null(self) -> bool {
        self as usize == 0
    }
    pub unsafe fn offset(self, count: isize) -> *mut T {
        intrinsics::ptr_offset(self as *const T, count) as *mut T
    }
    pub unsafe fn add(self, count: usize) -> *mut T {
        intrinsics::ptr_offset(self as *const T, count as isize) as *mut T
    }
    pub unsafe fn sub(self, count: usize) -> *mut T {
        intrinsics::ptr_offset(self as *const T, 0 - (count as isize)) as *mut T
    }
    pub fn wrapping_add(self, count: usize) -> *mut T {
        unsafe { intrinsics::ptr_offset(self as *const T, count as isize) as *mut T }
    }
    pub unsafe fn read(self) -> T {
        intrinsics::read(self as *const T)
    }
    pub unsafe fn write(self, val: T) {
        intrinsics::write(self, val)
    }
    pub unsafe fn offset_from(self, origin: *const T) -> isize {
        intrinsics::ptr_diff(self as *const T, origin)
    }
    pub fn cast<U>(self) -> *mut U {
        self as *mut U
    }
    pub fn cast_const(self) -> *const T {
        self as *const T
    }
    pub fn addr(self) -> usize {
        self as usize
    }
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        if self.is_null() { None } else { Some(&*self) }
    }
    pub unsafe fn as_mut<'a>(self) -> Option<&'a mut T> {
        if self.is_null() { None } else { Some(&mut *self) }
    }
    pub unsafe fn drop_in_place(self) {
        intrinsics::drop_in_place(self)
    }
}

impl<T: ?Sized> PartialEq for *const T {
    fn eq(&self, other: &*const T) -> bool {
        (*self as *const u8 as usize) == (*other as *const u8 as usize)
    }
}
impl<T: ?Sized> Eq for *const T {}
impl<T: ?Sized> PartialEq for *mut T {
    fn eq(&self, other: &*mut T) -> bool {
        (*self as *const u8 as usize) == (*other as *const u8 as usize)
    }
}
impl<T: ?Sized> Eq for *mut T {}

/// A non-null raw pointer (no niche in rcc; just a wrapper).
pub struct NonNull<T: ?Sized> {
    pointer: *const T,
}
impl<T> NonNull<T> {
    pub fn new(ptr: *mut T) -> Option<NonNull<T>> {
        if ptr.is_null() { None } else { Some(NonNull { pointer: ptr as *const T }) }
    }
    pub const unsafe fn new_unchecked(ptr: *mut T) -> NonNull<T> {
        NonNull { pointer: ptr as *const T }
    }
    pub fn dangling() -> NonNull<T> {
        NonNull { pointer: unsafe { intrinsics::align_of::<T>() } as *const T }
    }
    pub fn as_ptr(self) -> *mut T {
        self.pointer as *mut T
    }
    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        &*self.pointer
    }
    pub unsafe fn as_mut<'a>(&mut self) -> &'a mut T {
        &mut *(self.pointer as *mut T)
    }
}
impl<T: ?Sized> Clone for NonNull<T> {
    fn clone(&self) -> NonNull<T> {
        NonNull { pointer: self.pointer }
    }
}
impl<T: ?Sized> Copy for NonNull<T> {}
