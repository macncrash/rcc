//! Single-threaded reference counting.
//!
//! Layout: the value and the counts live in two allocations (`ptr` may be a
//! fat pointer for `Rc<str>` / `Rc<[T]>` / `Rc<dyn Trait>`), so that no
//! unsized struct is needed. `counts.strong` is the number of `Rc`s;
//! `counts.weak` the number of `Weak`s plus one while any `Rc` exists.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::ops::Deref;

struct Counts {
    strong: usize,
    weak: usize,
}

#[lang = "Rc"]
pub struct Rc<T: ?Sized> {
    ptr: *mut T,
    counts: *mut Counts,
}

pub struct Weak<T: ?Sized> {
    ptr: *mut T,
    counts: *mut Counts,
}

fn new_counts() -> *mut Counts {
    crate::boxed::Box::into_raw(crate::boxed::Box::new(Counts { strong: 1, weak: 1 }))
}

impl<T> Rc<T> {
    pub fn new(value: T) -> Rc<T> {
        Rc { ptr: crate::boxed::Box::into_raw(crate::boxed::Box::new(value)), counts: new_counts() }
    }
    /// Build an `Rc` whose value can hold a `Weak` to itself.
    pub fn new_cyclic<F: FnOnce(&Weak<T>) -> T>(data_fn: F) -> Rc<T> {
        unsafe {
            let ptr = crate::alloc::alloc_bytes(crate::mem::size_of::<T>(), crate::mem::align_of::<T>()) as *mut T;
            let counts = crate::boxed::Box::into_raw(crate::boxed::Box::new(Counts { strong: 0, weak: 1 }));
            let weak = Weak { ptr, counts };
            let value = data_fn(&weak);
            crate::ptr::write(ptr, value);
            (*counts).strong = 1;
            // the implicit weak reference held by the strong ones
            crate::mem::forget(weak);
            Rc { ptr, counts }
        }
    }
    pub fn try_unwrap(this: Rc<T>) -> Result<T, Rc<T>> {
        if Rc::strong_count(&this) == 1 {
            unsafe {
                let value = crate::ptr::read(this.ptr as *const T);
                let (ptr, counts) = (this.ptr, this.counts);
                crate::mem::forget(this);
                (*counts).strong = 0;
                crate::alloc::dealloc_bytes(ptr as *mut u8);
                release_weak(counts);
                Ok(value)
            }
        } else {
            Err(this)
        }
    }
    pub fn into_inner(this: Rc<T>) -> Option<T> {
        Rc::try_unwrap(this).ok()
    }
    pub fn unwrap_or_clone(this: Rc<T>) -> T
    where
        T: Clone,
    {
        match Rc::try_unwrap(this) {
            Ok(v) => v,
            Err(rc) => (*rc).clone(),
        }
    }
}

unsafe fn release_weak(counts: *mut Counts) {
    (*counts).weak -= 1;
    if (*counts).weak == 0 {
        crate::alloc::dealloc_bytes(counts as *mut u8);
    }
}

impl<T: ?Sized> Rc<T> {
    pub fn strong_count(this: &Rc<T>) -> usize {
        unsafe { (*this.counts).strong }
    }
    pub fn weak_count(this: &Rc<T>) -> usize {
        unsafe { (*this.counts).weak - 1 }
    }
    pub fn ptr_eq(this: &Rc<T>, other: &Rc<T>) -> bool {
        (this.counts as usize) == (other.counts as usize)
    }
    pub fn as_ptr(this: &Rc<T>) -> *const T {
        this.ptr as *const T
    }
    pub fn downgrade(this: &Rc<T>) -> Weak<T> {
        unsafe { (*this.counts).weak += 1 };
        Weak { ptr: this.ptr, counts: this.counts }
    }
    pub fn get_mut(this: &mut Rc<T>) -> Option<&mut T> {
        if Rc::strong_count(this) == 1 && Rc::weak_count(this) == 0 { unsafe { Some(&mut *this.ptr) } } else { None }
    }
    /// Build from a box (used for `Rc<str>`, `Rc<[T]>`).
    pub fn from_box(b: crate::boxed::Box<T>) -> Rc<T> {
        Rc { ptr: crate::boxed::Box::into_raw(b), counts: new_counts() }
    }
}

impl<T: Clone> Rc<T> {
    pub fn make_mut(this: &mut Rc<T>) -> &mut T {
        if Rc::strong_count(this) != 1 || Rc::weak_count(this) != 0 {
            *this = Rc::new((**this).clone());
        }
        unsafe { &mut *this.ptr }
    }
}

impl<T: ?Sized> Drop for Rc<T> {
    fn drop(&mut self) {
        unsafe {
            (*self.counts).strong -= 1;
            if (*self.counts).strong == 0 {
                crate::ptr::drop_in_place(self.ptr);
                crate::alloc::dealloc_bytes(self.ptr as *mut u8);
                release_weak(self.counts);
            }
        }
    }
}

impl<T: ?Sized> Clone for Rc<T> {
    fn clone(&self) -> Rc<T> {
        unsafe { (*self.counts).strong += 1 };
        Rc { ptr: self.ptr, counts: self.counts }
    }
}

impl<T: ?Sized> Deref for Rc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl<T: ?Sized> AsRef<T> for Rc<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}
impl<T: ?Sized> crate::borrow::Borrow<T> for Rc<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}
impl<T: Default> Default for Rc<T> {
    fn default() -> Rc<T> {
        Rc::new(T::default())
    }
}
impl<T: ?Sized + PartialEq> PartialEq for Rc<T> {
    fn eq(&self, other: &Rc<T>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}
impl<T: ?Sized + Eq> Eq for Rc<T> {}
impl<T: ?Sized + PartialOrd> PartialOrd for Rc<T> {
    fn partial_cmp(&self, other: &Rc<T>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}
impl<T: ?Sized + Ord> Ord for Rc<T> {
    fn cmp(&self, other: &Rc<T>) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: ?Sized + Hash> Hash for Rc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}
impl<T: ?Sized + fmt::Display> fmt::Display for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
impl<T: ?Sized + fmt::Debug> fmt::Debug for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
impl<T> From<T> for Rc<T> {
    fn from(t: T) -> Rc<T> {
        Rc::new(t)
    }
}
impl From<&str> for Rc<str> {
    fn from(s: &str) -> Rc<str> {
        Rc::from_box(crate::boxed::Box::from(s))
    }
}
impl From<crate::string::String> for Rc<str> {
    fn from(s: crate::string::String) -> Rc<str> {
        Rc::from_box(s.into_boxed_str())
    }
}
impl<T: Clone> From<&[T]> for Rc<[T]> {
    fn from(s: &[T]) -> Rc<[T]> {
        Rc::from_box(s.to_vec().into_boxed_slice())
    }
}
impl<T> From<crate::vec::Vec<T>> for Rc<[T]> {
    fn from(v: crate::vec::Vec<T>) -> Rc<[T]> {
        Rc::from_box(v.into_boxed_slice())
    }
}
impl<T: ?Sized> From<crate::boxed::Box<T>> for Rc<T> {
    fn from(b: crate::boxed::Box<T>) -> Rc<T> {
        Rc::from_box(b)
    }
}

impl<T> Weak<T> {
    pub fn new() -> Weak<T> {
        Weak { ptr: 0usize as *mut T, counts: 0usize as *mut Counts }
    }
}

impl<T: ?Sized> Weak<T> {
    pub fn upgrade(&self) -> Option<Rc<T>> {
        if (self.counts as usize) == 0 {
            return None;
        }
        unsafe {
            if (*self.counts).strong == 0 {
                None
            } else {
                (*self.counts).strong += 1;
                Some(Rc { ptr: self.ptr, counts: self.counts })
            }
        }
    }
    pub fn strong_count(&self) -> usize {
        if (self.counts as usize) == 0 { 0 } else { unsafe { (*self.counts).strong } }
    }
    pub fn weak_count(&self) -> usize {
        if (self.counts as usize) == 0 {
            return 0;
        }
        unsafe {
            let c = &*self.counts;
            if c.strong > 0 { c.weak - 1 } else { 0 }
        }
    }
    pub fn ptr_eq(&self, other: &Weak<T>) -> bool {
        (self.counts as usize) == (other.counts as usize)
    }
}

impl<T: ?Sized> Drop for Weak<T> {
    fn drop(&mut self) {
        if (self.counts as usize) != 0 {
            unsafe { release_weak(self.counts) }
        }
    }
}
impl<T: ?Sized> Clone for Weak<T> {
    fn clone(&self) -> Weak<T> {
        if (self.counts as usize) != 0 {
            unsafe { (*self.counts).weak += 1 };
        }
        Weak { ptr: self.ptr, counts: self.counts }
    }
}
impl<T> Default for Weak<T> {
    fn default() -> Weak<T> {
        Weak::new()
    }
}
impl<T: ?Sized> fmt::Debug for Weak<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(Weak)")
    }
}
