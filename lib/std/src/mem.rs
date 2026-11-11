use crate::intrinsics;

pub fn size_of<T>() -> usize {
    unsafe { intrinsics::size_of::<T>() }
}
pub fn align_of<T>() -> usize {
    unsafe { intrinsics::align_of::<T>() }
}
pub fn size_of_val<T>(_val: &T) -> usize {
    unsafe { intrinsics::size_of::<T>() }
}
pub fn needs_drop<T: ?Sized>() -> bool {
    unsafe { intrinsics::needs_drop::<T>() }
}
pub fn forget<T>(t: T) {
    unsafe { intrinsics::forget(t) }
}
pub fn drop<T>(_x: T) {}

pub fn swap<T>(a: &mut T, b: &mut T) {
    unsafe {
        let tmp = intrinsics::read(a as *mut T as *const T);
        intrinsics::copy_nonoverlapping(b as *mut T as *const T, a as *mut T, 1);
        intrinsics::write(b as *mut T, tmp);
    }
}
pub fn replace<T>(dest: &mut T, src: T) -> T {
    unsafe {
        let old = intrinsics::read(dest as *mut T as *const T);
        intrinsics::write(dest as *mut T, src);
        old
    }
}
pub fn take<T: Default>(dest: &mut T) -> T {
    replace(dest, T::default())
}
pub unsafe fn transmute<A, B>(x: A) -> B {
    intrinsics::transmute(x)
}
pub unsafe fn transmute_copy<A, B>(x: &A) -> B {
    intrinsics::read(x as *const A as *const B)
}
pub unsafe fn zeroed<T>() -> T {
    let mut v = MaybeUninit::<T>::uninit();
    intrinsics::write_bytes(v.as_mut_ptr(), 0, 1);
    v.assume_init()
}

/// Opaque identity of an enum variant.
pub struct Discriminant<T> {
    value: isize,
    marker: crate::marker::PhantomData<T>,
}
pub fn discriminant<T>(v: &T) -> Discriminant<T> {
    Discriminant { value: unsafe { intrinsics::discriminant_value(v as *const T) }, marker: crate::marker::PhantomData }
}
impl<T> Clone for Discriminant<T> {
    fn clone(&self) -> Discriminant<T> {
        Discriminant { value: self.value, marker: crate::marker::PhantomData }
    }
}
impl<T> Copy for Discriminant<T> {}
impl<T> PartialEq for Discriminant<T> {
    fn eq(&self, other: &Discriminant<T>) -> bool {
        self.value == other.value
    }
}
impl<T> Eq for Discriminant<T> {}
impl<T> crate::hash::Hash for Discriminant<T> {
    fn hash<H: crate::hash::Hasher>(&self, state: &mut H) {
        state.write_isize(self.value);
    }
}
impl<T> crate::fmt::Debug for Discriminant<T> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("Discriminant(")?;
        crate::fmt::Display::fmt(&self.value, f)?;
        f.write_str(")")
    }
}

/// A value that is never dropped automatically.
#[lang = "manually_drop"]
pub struct ManuallyDrop<T> {
    value: T,
}
impl<T> ManuallyDrop<T> {
    pub const fn new(value: T) -> ManuallyDrop<T> {
        ManuallyDrop { value }
    }
    pub fn into_inner(slot: ManuallyDrop<T>) -> T {
        unsafe {
            let v = intrinsics::read(&slot.value as *const T);
            v
        }
    }
    pub unsafe fn drop(slot: &mut ManuallyDrop<T>) {
        intrinsics::drop_in_place(&mut slot.value as *mut T)
    }
    pub unsafe fn take(slot: &mut ManuallyDrop<T>) -> T {
        intrinsics::read(&slot.value as *const T)
    }
}
impl<T> crate::ops::Deref for ManuallyDrop<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
impl<T> crate::ops::DerefMut for ManuallyDrop<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

/// Possibly uninitialized storage for a T (never dropped automatically).
pub struct MaybeUninit<T> {
    value: ManuallyDrop<T>,
}
impl<T> MaybeUninit<T> {
    pub fn new(value: T) -> MaybeUninit<T> {
        MaybeUninit { value: ManuallyDrop::new(value) }
    }
    pub fn uninit() -> MaybeUninit<T> {
        unsafe { uninit_value::<MaybeUninit<T>>() }
    }
    pub fn as_ptr(&self) -> *const T {
        self as *const MaybeUninit<T> as *const T
    }
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self as *mut MaybeUninit<T> as *mut T
    }
    pub unsafe fn assume_init(self) -> T {
        ManuallyDrop::into_inner(self.value)
    }
    pub fn write(&mut self, value: T) -> &mut T {
        unsafe {
            intrinsics::write(self.as_mut_ptr(), value);
            &mut *self.as_mut_ptr()
        }
    }
}

/// An uninitialized value of type T (the caller must initialize it before
/// use, and T must not need dropping while uninitialized).
unsafe fn uninit_value<T>() -> T {
    let x: T;
    loop {
        break;
    }
    // `x` is declared but never assigned: the generated C leaves it
    // uninitialized, and the drop flag machinery knows it holds nothing.
    intrinsics::read(&raw const x)
}
