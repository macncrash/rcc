//! `Once`, `OnceLock`, `LazyLock`.
//!
//! `Once` is an atomic state word (incomplete / running / complete);
//! threads that find it running wait (spin, then yield) until it completes.

use crate::cell::{Cell, UnsafeCell};
use crate::fmt;
use crate::ops::{Deref, DerefMut};
use crate::sync::atomic::{AtomicU8, Ordering};
use crate::sync::backoff;

const INCOMPLETE: u8 = 0;
const RUNNING: u8 = 1;
const COMPLETE: u8 = 2;

pub struct Once {
    state: AtomicU8,
}

unsafe impl Sync for Once {}
unsafe impl Send for Once {}

pub struct OnceState {
    poisoned: bool,
}

impl OnceState {
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }
}

pub const ONCE_INIT: Once = Once::new();

impl Once {
    pub const fn new() -> Once {
        Once { state: AtomicU8::new(INCOMPLETE) }
    }
    pub fn is_completed(&self) -> bool {
        self.state.load(Ordering::Acquire) == COMPLETE
    }
    /// Run `f` if this is the first call; otherwise wait until the first
    /// call has finished. Returns whether `f` ran.
    fn run<F: FnOnce()>(&self, f: F) -> bool {
        let mut spins = 0u32;
        loop {
            match self.state.compare_exchange(INCOMPLETE, RUNNING, Ordering::Acquire, Ordering::Acquire) {
                Ok(_) => {
                    f();
                    self.state.store(COMPLETE, Ordering::Release);
                    return true;
                }
                Err(COMPLETE) => return false,
                Err(_) => backoff(&mut spins),
            }
        }
    }
    pub fn call_once<F: FnOnce()>(&self, f: F) {
        if self.is_completed() {
            return;
        }
        self.run(f);
    }
    pub fn call_once_force<F: FnOnce(&OnceState)>(&self, f: F) {
        if self.is_completed() {
            return;
        }
        self.run(|| f(&OnceState { poisoned: false }));
    }
    pub fn wait(&self) {
        let mut spins = 0u32;
        while !self.is_completed() {
            backoff(&mut spins);
        }
    }
}

impl fmt::Debug for Once {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Once { .. }")
    }
}

// ---- OnceLock ----

pub struct OnceLock<T> {
    once: Once,
    value: UnsafeCell<Option<T>>,
}

unsafe impl<T: Sync + Send> Sync for OnceLock<T> {}
unsafe impl<T: Send> Send for OnceLock<T> {}

impl<T> OnceLock<T> {
    pub const fn new() -> OnceLock<T> {
        OnceLock { once: Once::new(), value: UnsafeCell::new(None) }
    }
    pub fn get(&self) -> Option<&T> {
        if self.once.is_completed() { unsafe { (*self.value.get()).as_ref() } } else { None }
    }
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.value.get_mut().as_mut()
    }
    pub fn wait(&self) -> &T {
        self.once.wait();
        self.get().unwrap()
    }
    pub fn set(&self, value: T) -> Result<(), T> {
        let mut slot = Some(value);
        if !self.once.is_completed() {
            let cell = &self.value;
            self.once.run(|| unsafe { *cell.get() = slot.take() });
        }
        match slot {
            None => Ok(()),
            Some(v) => Err(v),
        }
    }
    pub fn try_insert(&self, value: T) -> Result<&T, (&T, T)> {
        match self.set(value) {
            Ok(()) => Ok(self.get().unwrap()),
            Err(v) => Err((self.get().unwrap(), v)),
        }
    }
    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
        if !self.once.is_completed() {
            let cell = &self.value;
            self.once.run(|| {
                let v = f();
                unsafe { *cell.get() = Some(v) }
            });
        }
        unsafe { (*self.value.get()).as_ref().unwrap() }
    }
    pub fn get_or_try_init<F, E>(&self, f: F) -> Result<&T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        if let Some(v) = self.get() {
            return Ok(v);
        }
        match f() {
            Ok(v) => {
                let _ = self.set(v);
                Ok(self.get().unwrap())
            }
            Err(e) => Err(e),
        }
    }
    pub fn into_inner(self) -> Option<T> {
        self.value.into_inner()
    }
    pub fn take(&mut self) -> Option<T> {
        self.once = Once::new();
        self.value.get_mut().take()
    }
}

impl<T> Default for OnceLock<T> {
    fn default() -> OnceLock<T> {
        OnceLock::new()
    }
}

impl<T: Clone> Clone for OnceLock<T> {
    fn clone(&self) -> OnceLock<T> {
        let cell = OnceLock::new();
        if let Some(v) = self.get() {
            let _ = cell.set(v.clone());
        }
        cell
    }
}

impl<T> From<T> for OnceLock<T> {
    fn from(value: T) -> OnceLock<T> {
        let cell = OnceLock::new();
        let _ = cell.set(value);
        cell
    }
}

impl<T: PartialEq> PartialEq for OnceLock<T> {
    fn eq(&self, other: &OnceLock<T>) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq> Eq for OnceLock<T> {}

impl<T: fmt::Debug> fmt::Debug for OnceLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_tuple("OnceLock");
        match self.get() {
            Some(v) => d.field(v),
            None => d.field(&format_args!("<uninit>")),
        };
        d.finish()
    }
}

// ---- LazyLock ----

pub struct LazyLock<T, F = fn() -> T> {
    once: Once,
    init: Cell<Option<F>>,
    value: UnsafeCell<Option<T>>,
}

unsafe impl<T: Sync + Send, F: Send> Sync for LazyLock<T, F> {}

impl<T, F: FnOnce() -> T> LazyLock<T, F> {
    pub const fn new(f: F) -> LazyLock<T, F> {
        LazyLock { once: Once::new(), init: Cell::new(Some(f)), value: UnsafeCell::new(None) }
    }
    pub fn force(this: &LazyLock<T, F>) -> &T {
        if !this.once.is_completed() {
            this.once.run(|| {
                let f = match this.init.replace(None) {
                    Some(f) => f,
                    None => panic!("LazyLock instance has previously been poisoned"),
                };
                let v = f();
                unsafe { *this.value.get() = Some(v) }
            });
        }
        unsafe { (*this.value.get()).as_ref().unwrap() }
    }
    pub fn force_mut(this: &mut LazyLock<T, F>) -> &mut T {
        LazyLock::force(this);
        this.value.get_mut().as_mut().unwrap()
    }
    pub fn get(this: &LazyLock<T, F>) -> Option<&T> {
        if this.once.is_completed() { unsafe { (*this.value.get()).as_ref() } } else { None }
    }
    pub fn into_inner(this: LazyLock<T, F>) -> Result<T, F> {
        let LazyLock { once: _, init, value } = this;
        match value.into_inner() {
            Some(v) => Ok(v),
            None => Err(init.into_inner().unwrap()),
        }
    }
}

impl<T, F: FnOnce() -> T> Deref for LazyLock<T, F> {
    type Target = T;
    fn deref(&self) -> &T {
        LazyLock::force(self)
    }
}

impl<T, F: FnOnce() -> T> DerefMut for LazyLock<T, F> {
    fn deref_mut(&mut self) -> &mut T {
        LazyLock::force_mut(self)
    }
}

impl<T: Default> Default for LazyLock<T> {
    fn default() -> LazyLock<T> {
        LazyLock::new(T::default)
    }
}

impl<T: fmt::Debug, F> fmt::Debug for LazyLock<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_tuple("LazyLock");
        if self.once.is_completed() {
            match unsafe { (*self.value.get()).as_ref() } {
                Some(v) => d.field(v),
                None => d.field(&format_args!("<uninit>")),
            };
        } else {
            d.field(&format_args!("<uninit>"));
        }
        d.finish()
    }
}
