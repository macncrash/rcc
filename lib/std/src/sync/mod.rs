//! Synchronisation: `Arc`/`Weak`, atomics, `Mutex`, `RwLock`, `Condvar`,
//! `Once`, `OnceLock`, `LazyLock`. The OS is reached only through the
//! `rcc_thr_*` block of rt/rt.c (atomics, yield, sleep).

pub mod atomic;
mod arc;
mod mutex;
mod once;

pub use self::arc::{Arc, Weak};
pub use self::mutex::{
    Condvar, LockResult, Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError,
    TryLockResult, WaitTimeoutResult,
};
pub use self::once::{LazyLock, Once, OnceLock, OnceState, ONCE_INIT};

extern "C" {
    fn rcc_thr_yield();
    fn rcc_thr_sleep(secs: u64, nanos: u32);
}

/// One step of waiting for another thread: spin briefly, then yield the
/// CPU, then sleep 50us per step (locks here have no OS wait queue).
pub(crate) fn backoff(spins: &mut u32) {
    *spins += 1;
    if *spins < 64 {
        return;
    }
    if *spins < 2000 {
        unsafe { rcc_thr_yield() };
    } else {
        unsafe { rcc_thr_sleep(0, 50_000) };
    }
}
