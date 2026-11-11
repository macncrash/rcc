//! `Mutex`, `RwLock`, `Condvar` and the poisoning types.
//!
//! The locks are atomic words with spin-then-yield waiting (no OS mutex, so
//! `const fn new` works in statics). Poisoning cannot happen: a panic never
//! unwinds through a guard (a panicking spawned thread is abandoned without
//! running destructors, so a lock it holds stays locked, see thread/mod.rs);
//! the `LockResult` types exist for API compatibility and are always `Ok`.

use crate::cell::UnsafeCell;
use crate::fmt;
use crate::ops::{Deref, DerefMut};
use crate::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use crate::sync::backoff;

// ---- poisoning ----

pub struct PoisonError<T> {
    guard: T,
}

impl<T> PoisonError<T> {
    pub fn new(guard: T) -> PoisonError<T> {
        PoisonError { guard }
    }
    pub fn into_inner(self) -> T {
        self.guard
    }
    pub fn get_ref(&self) -> &T {
        &self.guard
    }
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.guard
    }
}

impl<T> fmt::Debug for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PoisonError { .. }")
    }
}

impl<T> fmt::Display for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("poisoned lock: another task failed inside")
    }
}

pub enum TryLockError<T> {
    Poisoned(PoisonError<T>),
    WouldBlock,
}

impl<T> fmt::Debug for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryLockError::Poisoned(..) => f.write_str("Poisoned(..)"),
            TryLockError::WouldBlock => f.write_str("WouldBlock"),
        }
    }
}

impl<T> fmt::Display for TryLockError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryLockError::Poisoned(..) => f.write_str("poisoned lock: another task failed inside"),
            TryLockError::WouldBlock => f.write_str("try_lock failed because the operation would block"),
        }
    }
}

impl<T> From<PoisonError<T>> for TryLockError<T> {
    fn from(err: PoisonError<T>) -> TryLockError<T> {
        TryLockError::Poisoned(err)
    }
}

pub type LockResult<G> = Result<G, PoisonError<G>>;
pub type TryLockResult<G> = Result<G, TryLockError<G>>;

// ---- Mutex ----

pub struct Mutex<T> {
    locked: AtomicU32,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

pub struct MutexGuard<'a, T: 'a> {
    lock: &'a Mutex<T>,
}

impl<T> Mutex<T> {
    pub const fn new(t: T) -> Mutex<T> {
        Mutex { locked: AtomicU32::new(0), data: UnsafeCell::new(t) }
    }
    fn raw_try_lock(&self) -> bool {
        self.locked.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_ok()
    }
    fn raw_lock(&self) {
        let mut spins = 0u32;
        while !self.raw_try_lock() {
            backoff(&mut spins);
        }
    }
    fn raw_unlock(&self) {
        self.locked.store(0, Ordering::Release);
    }
    pub fn lock(&self) -> LockResult<MutexGuard<'_, T>> {
        self.raw_lock();
        Ok(MutexGuard { lock: self })
    }
    pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> {
        if self.raw_try_lock() { Ok(MutexGuard { lock: self }) } else { Err(TryLockError::WouldBlock) }
    }
    pub fn is_poisoned(&self) -> bool {
        false
    }
    pub fn clear_poison(&self) {}
    pub fn into_inner(self) -> LockResult<T> {
        Ok(self.data.into_inner())
    }
    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        Ok(self.data.get_mut())
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Mutex<T> {
        Mutex::new(T::default())
    }
}

impl<T> From<T> for Mutex<T> {
    fn from(t: T) -> Mutex<T> {
        Mutex::new(t)
    }
}

impl<T: fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("Mutex");
        match self.try_lock() {
            Ok(guard) => {
                d.field("data", &&*guard);
            }
            Err(_) => {
                d.field("data", &format_args!("<locked>"));
            }
        }
        d.field("poisoned", &false);
        d.finish_non_exhaustive()
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.raw_unlock();
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for MutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: fmt::Display> fmt::Display for MutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

// ---- RwLock ----

/// `state`: number of readers, or `WRITER` while write-locked.
pub struct RwLock<T> {
    state: AtomicUsize,
    data: UnsafeCell<T>,
}

const WRITER: usize = usize::MAX;

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

pub struct RwLockReadGuard<'a, T: 'a> {
    lock: &'a RwLock<T>,
}

pub struct RwLockWriteGuard<'a, T: 'a> {
    lock: &'a RwLock<T>,
}

impl<T> RwLock<T> {
    pub const fn new(t: T) -> RwLock<T> {
        RwLock { state: AtomicUsize::new(0), data: UnsafeCell::new(t) }
    }
    fn raw_try_read(&self) -> bool {
        let s = self.state.load(Ordering::Relaxed);
        s != WRITER && s != WRITER - 1 && self.state.compare_exchange(s, s + 1, Ordering::Acquire, Ordering::Relaxed).is_ok()
    }
    fn raw_try_write(&self) -> bool {
        self.state.compare_exchange(0, WRITER, Ordering::Acquire, Ordering::Relaxed).is_ok()
    }
    pub fn read(&self) -> LockResult<RwLockReadGuard<'_, T>> {
        let mut spins = 0u32;
        while !self.raw_try_read() {
            backoff(&mut spins);
        }
        Ok(RwLockReadGuard { lock: self })
    }
    pub fn try_read(&self) -> TryLockResult<RwLockReadGuard<'_, T>> {
        if self.raw_try_read() { Ok(RwLockReadGuard { lock: self }) } else { Err(TryLockError::WouldBlock) }
    }
    pub fn write(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
        let mut spins = 0u32;
        while !self.raw_try_write() {
            backoff(&mut spins);
        }
        Ok(RwLockWriteGuard { lock: self })
    }
    pub fn try_write(&self) -> TryLockResult<RwLockWriteGuard<'_, T>> {
        if self.raw_try_write() { Ok(RwLockWriteGuard { lock: self }) } else { Err(TryLockError::WouldBlock) }
    }
    pub fn is_poisoned(&self) -> bool {
        false
    }
    pub fn clear_poison(&self) {}
    pub fn into_inner(self) -> LockResult<T> {
        Ok(self.data.into_inner())
    }
    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        Ok(self.data.get_mut())
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> RwLock<T> {
        RwLock::new(T::default())
    }
}

impl<T> From<T> for RwLock<T> {
    fn from(t: T) -> RwLock<T> {
        RwLock::new(t)
    }
}

impl<T: fmt::Debug> fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("RwLock");
        match self.try_read() {
            Ok(guard) => {
                d.field("data", &&*guard);
            }
            Err(_) => {
                d.field("data", &format_args!("<locked>"));
            }
        }
        d.field("poisoned", &false);
        d.finish_non_exhaustive()
    }
}

impl<'a, T> Deref for RwLockReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_sub(1, Ordering::Release);
    }
}

impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.state.store(0, Ordering::Release);
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for RwLockReadGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for RwLockWriteGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

// ---- Condvar ----

/// A sequence number bumped by every notification; waiters poll it.
pub struct Condvar {
    seq: AtomicU32,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct WaitTimeoutResult(bool);

impl WaitTimeoutResult {
    pub fn timed_out(&self) -> bool {
        self.0
    }
}

impl Condvar {
    pub const fn new() -> Condvar {
        Condvar { seq: AtomicU32::new(0) }
    }
    pub fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> LockResult<MutexGuard<'a, T>> {
        let seen = self.seq.load(Ordering::Acquire);
        let mutex = guard.lock;
        drop(guard);
        let mut spins = 0u32;
        while self.seq.load(Ordering::Acquire) == seen {
            backoff(&mut spins);
        }
        mutex.lock()
    }
    pub fn wait_while<'a, T, F>(&self, mut guard: MutexGuard<'a, T>, mut condition: F) -> LockResult<MutexGuard<'a, T>>
    where
        F: FnMut(&mut T) -> bool,
    {
        while condition(&mut *guard) {
            guard = match self.wait(guard) {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
        }
        Ok(guard)
    }
    pub fn notify_one(&self) {
        self.seq.fetch_add(1, Ordering::Release);
    }
    pub fn notify_all(&self) {
        self.seq.fetch_add(1, Ordering::Release);
    }
}

impl Default for Condvar {
    fn default() -> Condvar {
        Condvar::new()
    }
}

impl fmt::Debug for Condvar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Condvar { .. }")
    }
}
