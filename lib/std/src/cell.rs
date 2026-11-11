//! Shared mutability: `Cell`, `RefCell`, `OnceCell`, `UnsafeCell`.

use crate::cmp::Ordering;
use crate::fmt;
use crate::ops::{Deref, DerefMut};

pub struct UnsafeCell<T> {
    value: T,
}

impl<T> UnsafeCell<T> {
    pub const fn new(value: T) -> UnsafeCell<T> {
        UnsafeCell { value }
    }
    pub fn get(&self) -> *mut T {
        &self.value as *const T as *mut T
    }
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }
    pub fn into_inner(self) -> T {
        self.value
    }
}

pub struct Cell<T> {
    value: UnsafeCell<T>,
}

impl<T> Cell<T> {
    pub const fn new(value: T) -> Cell<T> {
        Cell { value: UnsafeCell::new(value) }
    }
    pub fn set(&self, val: T) {
        let old = self.replace(val);
        drop(old);
    }
    pub fn replace(&self, val: T) -> T {
        unsafe { crate::ptr::replace(self.value.get(), val) }
    }
    pub fn swap(&self, other: &Cell<T>) {
        unsafe { crate::ptr::swap(self.value.get(), other.value.get()) }
    }
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }
    pub fn take(&self) -> T
    where
        T: Default,
    {
        self.replace(T::default())
    }
}

impl<T: Copy> Cell<T> {
    pub fn get(&self) -> T {
        unsafe { *self.value.get() }
    }
    pub fn update<F: FnOnce(T) -> T>(&self, f: F) {
        self.set(f(self.get()));
    }
}

impl<T: Copy> Clone for Cell<T> {
    fn clone(&self) -> Cell<T> {
        Cell::new(self.get())
    }
}
impl<T: Default> Default for Cell<T> {
    fn default() -> Cell<T> {
        Cell::new(T::default())
    }
}
impl<T: Copy + PartialEq> PartialEq for Cell<T> {
    fn eq(&self, other: &Cell<T>) -> bool {
        self.get() == other.get()
    }
}
impl<T: Copy + Eq> Eq for Cell<T> {}
impl<T: Copy + PartialOrd> PartialOrd for Cell<T> {
    fn partial_cmp(&self, other: &Cell<T>) -> Option<Ordering> {
        self.get().partial_cmp(&other.get())
    }
}
impl<T: Copy + fmt::Debug> fmt::Debug for Cell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cell").field("value", &self.get()).finish()
    }
}
impl<T> From<T> for Cell<T> {
    fn from(t: T) -> Cell<T> {
        Cell::new(t)
    }
}

#[derive(Debug)]
pub struct BorrowError;
#[derive(Debug)]
pub struct BorrowMutError;

impl fmt::Display for BorrowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("already mutably borrowed")
    }
}
impl fmt::Display for BorrowMutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("already borrowed")
    }
}

/// `borrow` counts shared borrows (> 0) or is -1 while mutably borrowed.
pub struct RefCell<T> {
    borrow: Cell<isize>,
    value: UnsafeCell<T>,
}

impl<T> RefCell<T> {
    pub const fn new(value: T) -> RefCell<T> {
        RefCell { borrow: Cell::new(0), value: UnsafeCell::new(value) }
    }
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
    pub fn replace(&self, t: T) -> T {
        crate::mem::replace(&mut *self.borrow_mut(), t)
    }
    pub fn replace_with<F: FnOnce(&mut T) -> T>(&self, f: F) -> T {
        let mut b = self.borrow_mut();
        let new = f(&mut *b);
        crate::mem::replace(&mut *b, new)
    }
    pub fn swap(&self, other: &RefCell<T>) {
        crate::mem::swap(&mut *self.borrow_mut(), &mut *other.borrow_mut())
    }
    pub fn borrow(&self) -> Ref<'_, T> {
        match self.try_borrow() {
            Ok(b) => b,
            Err(_) => panic!("already mutably borrowed: BorrowError"),
        }
    }
    pub fn try_borrow(&self) -> Result<Ref<'_, T>, BorrowError> {
        let b = self.borrow.get();
        if b < 0 {
            return Err(BorrowError);
        }
        self.borrow.set(b + 1);
        Ok(Ref { value: unsafe { &*self.value.get() }, borrow: &self.borrow })
    }
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        match self.try_borrow_mut() {
            Ok(b) => b,
            Err(_) => panic!("already borrowed: BorrowMutError"),
        }
    }
    pub fn try_borrow_mut(&self) -> Result<RefMut<'_, T>, BorrowMutError> {
        if self.borrow.get() != 0 {
            return Err(BorrowMutError);
        }
        self.borrow.set(-1);
        Ok(RefMut { value: unsafe { &mut *self.value.get() }, borrow: &self.borrow })
    }
    pub fn as_ptr(&self) -> *mut T {
        self.value.get()
    }
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }
    pub fn take(&self) -> T
    where
        T: Default,
    {
        self.replace(T::default())
    }
}

impl<T: Clone> Clone for RefCell<T> {
    fn clone(&self) -> RefCell<T> {
        RefCell::new(self.borrow().clone())
    }
}
impl<T: Default> Default for RefCell<T> {
    fn default() -> RefCell<T> {
        RefCell::new(T::default())
    }
}
impl<T: PartialEq> PartialEq for RefCell<T> {
    fn eq(&self, other: &RefCell<T>) -> bool {
        *self.borrow() == *other.borrow()
    }
}
impl<T: Eq> Eq for RefCell<T> {}
impl<T: fmt::Debug> fmt::Debug for RefCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.try_borrow() {
            Ok(b) => f.debug_struct("RefCell").field("value", &*b).finish(),
            Err(_) => f.write_str("RefCell { value: <borrowed> }"),
        }
    }
}
impl<T> From<T> for RefCell<T> {
    fn from(t: T) -> RefCell<T> {
        RefCell::new(t)
    }
}

pub struct Ref<'b, T: ?Sized> {
    value: &'b T,
    borrow: &'b Cell<isize>,
}

impl<'b, T: ?Sized> Ref<'b, T> {
    pub fn clone(orig: &Ref<'b, T>) -> Ref<'b, T> {
        orig.borrow.set(orig.borrow.get() + 1);
        Ref { value: orig.value, borrow: orig.borrow }
    }
    pub fn map<U: ?Sized, F: FnOnce(&T) -> &U>(orig: Ref<'b, T>, f: F) -> Ref<'b, U> {
        let value = f(orig.value);
        let borrow = orig.borrow;
        crate::mem::forget(orig);
        Ref { value, borrow }
    }
}
impl<'b, T: ?Sized> Ref<'b, T> {
    pub fn map_split<U: ?Sized, V: ?Sized, F: FnOnce(&T) -> (&U, &V)>(orig: Ref<'b, T>, f: F) -> (Ref<'b, U>, Ref<'b, V>) {
        let (a, b) = f(orig.value);
        let borrow = orig.borrow;
        borrow.set(borrow.get() + 1);
        crate::mem::forget(orig);
        (Ref { value: a, borrow }, Ref { value: b, borrow })
    }
    pub fn filter_map<U: ?Sized, F: FnOnce(&T) -> Option<&U>>(orig: Ref<'b, T>, f: F) -> Result<Ref<'b, U>, Ref<'b, T>> {
        match f(orig.value) {
            Some(value) => {
                let borrow = orig.borrow;
                crate::mem::forget(orig);
                Ok(Ref { value, borrow })
            }
            None => Err(orig),
        }
    }
}

impl<'b, T: ?Sized> Drop for Ref<'b, T> {
    fn drop(&mut self) {
        self.borrow.set(self.borrow.get() - 1);
    }
}
impl<'b, T: ?Sized> Deref for Ref<'b, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.value
    }
}
impl<'b, T: ?Sized + fmt::Debug> fmt::Debug for Ref<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.value, f)
    }
}
impl<'b, T: ?Sized + fmt::Display> fmt::Display for Ref<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.value, f)
    }
}

pub struct RefMut<'b, T: ?Sized> {
    value: &'b mut T,
    borrow: &'b Cell<isize>,
}

impl<'b, T: ?Sized> RefMut<'b, T> {
    pub fn map<U: ?Sized, F: FnOnce(&mut T) -> &mut U>(orig: RefMut<'b, T>, f: F) -> RefMut<'b, U> {
        unsafe {
            let p = crate::ptr::read(&orig.value as *const &'b mut T);
            let borrow = orig.borrow;
            crate::mem::forget(orig);
            RefMut { value: f(p), borrow }
        }
    }
}
impl<'b, T: ?Sized> Drop for RefMut<'b, T> {
    fn drop(&mut self) {
        self.borrow.set(0);
    }
}
impl<'b, T: ?Sized> Deref for RefMut<'b, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.value
    }
}
impl<'b, T: ?Sized> DerefMut for RefMut<'b, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.value
    }
}
impl<'b, T: ?Sized + fmt::Debug> fmt::Debug for RefMut<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.value, f)
    }
}
impl<'b, T: ?Sized + fmt::Display> fmt::Display for RefMut<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.value, f)
    }
}

/// A cell written at most once.
pub struct OnceCell<T> {
    inner: UnsafeCell<Option<T>>,
}

impl<T> OnceCell<T> {
    pub const fn new() -> OnceCell<T> {
        OnceCell { inner: UnsafeCell::new(None) }
    }
    pub fn get(&self) -> Option<&T> {
        unsafe { (*self.inner.get()).as_ref() }
    }
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.inner.get_mut().as_mut()
    }
    pub fn set(&self, value: T) -> Result<(), T> {
        if self.get().is_some() {
            return Err(value);
        }
        unsafe { *self.inner.get() = Some(value) };
        Ok(())
    }
    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
        if self.get().is_none() {
            let v = f();
            unsafe { *self.inner.get() = Some(v) };
        }
        self.get().unwrap()
    }
    pub fn into_inner(self) -> Option<T> {
        self.inner.into_inner()
    }
    pub fn take(&mut self) -> Option<T> {
        self.inner.get_mut().take()
    }
}
impl<T> Default for OnceCell<T> {
    fn default() -> OnceCell<T> {
        OnceCell::new()
    }
}
