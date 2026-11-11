use crate::fmt;
use crate::hash::Hash;
use crate::intrinsics;
use crate::ops::{Deref, DerefMut};

/// A heap allocation. Layout contract with the compiler (cgen.c): exactly
/// one field, the raw pointer (fat for unsized T). Deref, moving out of
/// `*b`, unsizing coercions and dropping (drop contents, free) are built in.
#[lang = "owned_box"]
pub struct Box<T: ?Sized>(*mut T);

impl<T> Box<T> {
    pub fn new(x: T) -> Box<T> {
        unsafe {
            let p = crate::alloc::alloc_bytes(intrinsics::size_of::<T>(), intrinsics::align_of::<T>()) as *mut T;
            intrinsics::write(p, x);
            Box(p)
        }
    }
    pub fn into_inner(b: Box<T>) -> T {
        *b
    }
}

impl<T: ?Sized> Box<T> {
    pub fn into_raw(b: Box<T>) -> *mut T {
        let p = b.0;
        crate::mem::forget(b);
        p
    }
    pub unsafe fn from_raw(raw: *mut T) -> Box<T> {
        Box(raw)
    }
    pub fn leak<'a>(b: Box<T>) -> &'a mut T {
        unsafe { &mut *Box::into_raw(b) }
    }
    pub fn as_ptr(b: &Box<T>) -> *const T {
        b.0 as *const T
    }
}

impl<T: ?Sized> Deref for Box<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &**self
    }
}
impl<T: ?Sized> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: Clone> Clone for Box<T> {
    fn clone(&self) -> Box<T> {
        Box::new((**self).clone())
    }
}
impl<T: Default> Default for Box<T> {
    fn default() -> Box<T> {
        Box::new(T::default())
    }
}
impl<T: ?Sized + PartialEq> PartialEq for Box<T> {
    fn eq(&self, other: &Box<T>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}
impl<T: ?Sized + Eq> Eq for Box<T> {}
impl<T: ?Sized + PartialOrd> PartialOrd for Box<T> {
    fn partial_cmp(&self, other: &Box<T>) -> Option<crate::cmp::Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}
impl<T: ?Sized + Ord> Ord for Box<T> {
    fn cmp(&self, other: &Box<T>) -> crate::cmp::Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: ?Sized + fmt::Display> fmt::Display for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
impl<T: ?Sized + fmt::Debug> fmt::Debug for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
impl<T: ?Sized + crate::hash::Hash> crate::hash::Hash for Box<T> {
    fn hash<H: crate::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}
impl<T> From<T> for Box<T> {
    fn from(t: T) -> Box<T> {
        Box::new(t)
    }
}
impl<T: ?Sized> AsRef<T> for Box<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}
impl<T: ?Sized> AsMut<T> for Box<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}
impl<T: ?Sized> crate::borrow::Borrow<T> for Box<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}
impl<I: Iterator + ?Sized> Iterator for Box<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        (**self).next()
    }
}

// Calling boxed closures. For `Box<dyn FnOnce>` the compiler passes the box
// itself to the vtable entry, which unboxes the callee (mir_build.c).
impl<A, F: ?Sized + FnOnce<A>> FnOnce<A> for Box<F> {
    type Output = F::Output;
    fn call_once(self, args: A) -> F::Output {
        <F as FnOnce<A>>::call_once(*self, args)
    }
}
impl<A, F: ?Sized + Fn<A>> Fn<A> for Box<F> {
    fn call(&self, args: A) -> F::Output {
        (**self).call(args)
    }
}
impl<A, F: ?Sized + FnMut<A>> FnMut<A> for Box<F> {
    fn call_mut(&mut self, args: A) -> F::Output {
        (**self).call_mut(args)
    }
}

impl From<&str> for Box<str> {
    fn from(s: &str) -> Box<str> {
        crate::string::String::from(s).into_boxed_str()
    }
}
impl Clone for Box<str> {
    fn clone(&self) -> Box<str> {
        let s: &str = &**self;
        Box::from(s)
    }
}
impl<T: Clone> Clone for Box<[T]> {
    fn clone(&self) -> Box<[T]> {
        (**self).to_vec().into_boxed_slice()
    }
}

// `collect()` into a boxed slice (added with std::collections).
impl<T> FromIterator<T> for Box<[T]> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Box<[T]> {
        let v: Vec<T> = Vec::from_iter(iter);
        v.into_boxed_slice()
    }
}
