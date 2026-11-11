//! `LocalKey`: what `thread_local!` lowers to
//! (`static N: LocalKey<T> = LocalKey::new(__rcc_tls_init_N);`).
//!
//! The key is the address of the `static`. Each thread has its own table in
//! rt/rt.c (a `_Thread_local` hash map from key address to a boxed value);
//! a value is created by the init fn on the thread's first access and
//! dropped when a spawned thread exits (in reverse order of creation).

use crate::cell::{Cell, RefCell};
use crate::fmt;

extern "C" {
    fn rcc_thr_tls_get(key: *const u8) -> *mut u8;
    fn rcc_thr_tls_set(key: *const u8, val: *mut u8, dtor: fn(*mut u8) -> i32);
}

pub struct LocalKey<T: 'static> {
    init: fn() -> T,
}

unsafe impl<T: 'static> Sync for LocalKey<T> {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AccessError;

impl fmt::Debug for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccessError")
    }
}

impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("already destroyed")
    }
}

fn destroy_value<T>(p: *mut u8) -> i32 {
    unsafe { drop(crate::boxed::Box::from_raw(p as *mut T)) };
    0
}

impl<T: 'static> LocalKey<T> {
    pub const fn new(init: fn() -> T) -> LocalKey<T> {
        LocalKey { init }
    }

    fn key(&'static self) -> *const u8 {
        self as *const LocalKey<T> as *const u8
    }

    /// This thread's value, or null if it was not initialised yet.
    fn try_get(&'static self) -> *const T {
        unsafe { rcc_thr_tls_get(self.key()) as *const T }
    }

    /// Install `value` as this thread's value (the key must be unset).
    fn install(&'static self, value: T) -> *const T {
        let p = crate::boxed::Box::into_raw(crate::boxed::Box::new(value));
        unsafe { rcc_thr_tls_set(self.key(), p as *mut u8, destroy_value::<T>) };
        p as *const T
    }

    fn get_ptr(&'static self) -> *const T {
        let p = self.try_get();
        if !p.is_null() {
            return p;
        }
        let value = (self.init)();
        // the initializer may itself have initialised this key
        let p = self.try_get();
        if !p.is_null() {
            drop(value);
            return p;
        }
        self.install(value)
    }

    pub fn with<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let p = self.get_ptr();
        f(unsafe { &*p })
    }

    pub fn try_with<F, R>(&'static self, f: F) -> Result<R, AccessError>
    where
        F: FnOnce(&T) -> R,
    {
        Ok(self.with(f))
    }
}

impl<T: 'static> LocalKey<Cell<T>> {
    /// Like real std: does not run the initializer if the key is unset.
    pub fn set(&'static self, value: T) {
        let p = self.try_get();
        if p.is_null() {
            self.install(Cell::new(value));
        } else {
            unsafe { (*p).set(value) }
        }
    }
    pub fn get(&'static self) -> T
    where
        T: Copy,
    {
        self.with(|c| c.get())
    }
    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        self.with(|c| c.take())
    }
    pub fn replace(&'static self, value: T) -> T {
        self.with(|c| c.replace(value))
    }
}

impl<T: 'static> LocalKey<RefCell<T>> {
    pub fn with_borrow<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.with(|c| f(&*c.borrow()))
    }
    pub fn with_borrow_mut<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        self.with(|c| f(&mut *c.borrow_mut()))
    }
    /// Like real std: does not run the initializer if the key is unset.
    pub fn set(&'static self, value: T) {
        let p = self.try_get();
        if p.is_null() {
            self.install(RefCell::new(value));
        } else {
            unsafe { *(*p).borrow_mut() = value }
        }
    }
    pub fn take(&'static self) -> T
    where
        T: Default,
    {
        self.with(|c| c.take())
    }
    pub fn replace(&'static self, value: T) -> T {
        self.with(|c| c.replace(value))
    }
}

impl<T: 'static> fmt::Debug for LocalKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LocalKey { .. }")
    }
}
