//! Atomically reference-counted pointers. Same representation as `Rc`
//! (see rc.rs): `ptr` (field 0, possibly fat, which the compiler's
//! unsizing coercion relies on) and the counts in a second allocation;
//! the counts are atomics.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::ops::Deref;
use crate::sync::atomic::AtomicUsize;
use crate::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst};

struct Counts {
    strong: AtomicUsize,
    /// number of `Weak`s, plus one while any `Arc` exists
    weak: AtomicUsize,
}

#[lang = "Arc"]
pub struct Arc<T: ?Sized> {
    ptr: *mut T,
    counts: *mut Counts,
}

pub struct Weak<T: ?Sized> {
    ptr: *mut T,
    counts: *mut Counts,
}

unsafe impl<T: ?Sized + Sync + Send> Send for Arc<T> {}
unsafe impl<T: ?Sized + Sync + Send> Sync for Arc<T> {}
unsafe impl<T: ?Sized + Sync + Send> Send for Weak<T> {}
unsafe impl<T: ?Sized + Sync + Send> Sync for Weak<T> {}

fn new_counts(strong: usize) -> *mut Counts {
    crate::boxed::Box::into_raw(crate::boxed::Box::new(Counts { strong: AtomicUsize::new(strong), weak: AtomicUsize::new(1) }))
}

unsafe fn release_weak(counts: *mut Counts) {
    if (*counts).weak.fetch_sub(1, Release) == 1 {
        crate::alloc::dealloc_bytes(counts as *mut u8);
    }
}

impl<T> Arc<T> {
    pub fn new(value: T) -> Arc<T> {
        Arc { ptr: crate::boxed::Box::into_raw(crate::boxed::Box::new(value)), counts: new_counts(1) }
    }
    pub fn new_cyclic<F: FnOnce(&Weak<T>) -> T>(data_fn: F) -> Arc<T> {
        unsafe {
            let ptr = crate::alloc::alloc_bytes(crate::mem::size_of::<T>(), crate::mem::align_of::<T>()) as *mut T;
            let counts = new_counts(0);
            let weak = Weak { ptr, counts };
            let value = data_fn(&weak);
            crate::ptr::write(ptr, value);
            (*counts).strong.store(1, Release);
            // the implicit weak reference held by the strong ones
            crate::mem::forget(weak);
            Arc { ptr, counts }
        }
    }
    pub fn try_unwrap(this: Arc<T>) -> Result<T, Arc<T>> {
        unsafe {
            if (*this.counts).strong.compare_exchange(1, 0, Relaxed, Relaxed).is_err() {
                return Err(this);
            }
            let value = crate::ptr::read(this.ptr as *const T);
            let (ptr, counts) = (this.ptr, this.counts);
            crate::mem::forget(this);
            crate::alloc::dealloc_bytes(ptr as *mut u8);
            release_weak(counts);
            Ok(value)
        }
    }
    pub fn into_inner(this: Arc<T>) -> Option<T> {
        Arc::try_unwrap(this).ok()
    }
    pub fn unwrap_or_clone(this: Arc<T>) -> T
    where
        T: Clone,
    {
        match Arc::try_unwrap(this) {
            Ok(v) => v,
            Err(arc) => (*arc).clone(),
        }
    }
}

impl<T: ?Sized> Arc<T> {
    pub fn strong_count(this: &Arc<T>) -> usize {
        unsafe { (*this.counts).strong.load(SeqCst) }
    }
    pub fn weak_count(this: &Arc<T>) -> usize {
        unsafe { (*this.counts).weak.load(SeqCst) - 1 }
    }
    pub fn ptr_eq(this: &Arc<T>, other: &Arc<T>) -> bool {
        (this.counts as usize) == (other.counts as usize)
    }
    pub fn as_ptr(this: &Arc<T>) -> *const T {
        this.ptr as *const T
    }
    pub fn downgrade(this: &Arc<T>) -> Weak<T> {
        unsafe { (*this.counts).weak.fetch_add(1, Relaxed) };
        Weak { ptr: this.ptr, counts: this.counts }
    }
    pub fn get_mut(this: &mut Arc<T>) -> Option<&mut T> {
        if Arc::strong_count(this) == 1 && Arc::weak_count(this) == 0 { unsafe { Some(&mut *this.ptr) } } else { None }
    }
    /// Build from a box (used for `Arc<str>`, `Arc<[T]>`).
    pub fn from_box(b: crate::boxed::Box<T>) -> Arc<T> {
        Arc { ptr: crate::boxed::Box::into_raw(b), counts: new_counts(1) }
    }
}

impl<T: Clone> Arc<T> {
    pub fn make_mut(this: &mut Arc<T>) -> &mut T {
        if Arc::strong_count(this) != 1 || Arc::weak_count(this) != 0 {
            *this = Arc::new((**this).clone());
        }
        unsafe { &mut *this.ptr }
    }
}

impl<T: ?Sized> Drop for Arc<T> {
    fn drop(&mut self) {
        unsafe {
            if (*self.counts).strong.fetch_sub(1, Release) == 1 {
                crate::sync::atomic::fence(Acquire);
                crate::ptr::drop_in_place(self.ptr);
                crate::alloc::dealloc_bytes(self.ptr as *mut u8);
                release_weak(self.counts);
            }
        }
    }
}

impl<T: ?Sized> Clone for Arc<T> {
    fn clone(&self) -> Arc<T> {
        unsafe { (*self.counts).strong.fetch_add(1, Relaxed) };
        Arc { ptr: self.ptr, counts: self.counts }
    }
}

impl<T: ?Sized> Deref for Arc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl<T: ?Sized> AsRef<T> for Arc<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}
impl<T: ?Sized> crate::borrow::Borrow<T> for Arc<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}
impl<T: Default> Default for Arc<T> {
    fn default() -> Arc<T> {
        Arc::new(T::default())
    }
}
impl<T: ?Sized + PartialEq> PartialEq for Arc<T> {
    fn eq(&self, other: &Arc<T>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}
impl<T: ?Sized + Eq> Eq for Arc<T> {}
impl<T: ?Sized + PartialOrd> PartialOrd for Arc<T> {
    fn partial_cmp(&self, other: &Arc<T>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}
impl<T: ?Sized + Ord> Ord for Arc<T> {
    fn cmp(&self, other: &Arc<T>) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: ?Sized + Hash> Hash for Arc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}
impl<T: ?Sized + fmt::Display> fmt::Display for Arc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
impl<T: ?Sized + fmt::Debug> fmt::Debug for Arc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
impl<T> From<T> for Arc<T> {
    fn from(t: T) -> Arc<T> {
        Arc::new(t)
    }
}
impl From<&str> for Arc<str> {
    fn from(s: &str) -> Arc<str> {
        Arc::from_box(crate::boxed::Box::from(s))
    }
}
impl From<crate::string::String> for Arc<str> {
    fn from(s: crate::string::String) -> Arc<str> {
        Arc::from_box(s.into_boxed_str())
    }
}
impl<T: Clone> From<&[T]> for Arc<[T]> {
    fn from(s: &[T]) -> Arc<[T]> {
        Arc::from_box(s.to_vec().into_boxed_slice())
    }
}
impl<T> From<crate::vec::Vec<T>> for Arc<[T]> {
    fn from(v: crate::vec::Vec<T>) -> Arc<[T]> {
        Arc::from_box(v.into_boxed_slice())
    }
}
impl<T: ?Sized> From<crate::boxed::Box<T>> for Arc<T> {
    fn from(b: crate::boxed::Box<T>) -> Arc<T> {
        Arc::from_box(b)
    }
}

impl<T> Weak<T> {
    pub fn new() -> Weak<T> {
        Weak { ptr: 0usize as *mut T, counts: 0usize as *mut Counts }
    }
}

impl<T: ?Sized> Weak<T> {
    pub fn upgrade(&self) -> Option<Arc<T>> {
        if (self.counts as usize) == 0 {
            return None;
        }
        unsafe {
            let strong = &(*self.counts).strong;
            let mut n = strong.load(Relaxed);
            loop {
                if n == 0 {
                    return None;
                }
                match strong.compare_exchange_weak(n, n + 1, Acquire, Relaxed) {
                    Ok(_) => return Some(Arc { ptr: self.ptr, counts: self.counts }),
                    Err(old) => n = old,
                }
            }
        }
    }
    pub fn strong_count(&self) -> usize {
        if (self.counts as usize) == 0 { 0 } else { unsafe { (*self.counts).strong.load(SeqCst) } }
    }
    pub fn weak_count(&self) -> usize {
        if (self.counts as usize) == 0 {
            return 0;
        }
        unsafe {
            let c = &*self.counts;
            if c.strong.load(SeqCst) > 0 { c.weak.load(SeqCst) - 1 } else { 0 }
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
            unsafe { (*self.counts).weak.fetch_add(1, Relaxed) };
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
