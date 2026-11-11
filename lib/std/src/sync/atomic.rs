//! Atomic integers, `AtomicBool`, `AtomicPtr`.
//!
//! Every operation is one call of `rcc_thr_atomic` in rt/rt.c, which uses
//! the `__atomic` builtins (supported by clang and ccc) and is always
//! sequentially consistent, whatever `Ordering` is asked for (stronger is
//! always correct). `fetch_max`/`fetch_min`/`fetch_nand`/`fetch_update`
//! are compare-exchange loops here.

use crate::cell::UnsafeCell;
use crate::fmt;

extern "C" {
    fn rcc_thr_atomic(p: *mut u8, size: i32, op: i32, v: u64, w: u64) -> u64;
}

const OP_LOAD: i32 = 0;
const OP_STORE: i32 = 1;
const OP_SWAP: i32 = 2;
const OP_ADD: i32 = 3;
const OP_SUB: i32 = 4;
const OP_AND: i32 = 5;
const OP_OR: i32 = 6;
const OP_XOR: i32 = 7;
const OP_CAS: i32 = 8;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ordering {
    Relaxed,
    Release,
    Acquire,
    AcqRel,
    SeqCst,
}

/// A full memory barrier (every ordering is treated as `SeqCst`).
pub fn fence(_order: Ordering) {
    let mut word: u64 = 0;
    unsafe { rcc_thr_atomic(&mut word as *mut u64 as *mut u8, 8, OP_SWAP, 0, 0) };
}

pub fn compiler_fence(order: Ordering) {
    fence(order)
}

pub fn spin_loop_hint() {}

macro_rules! atomic_int {
    ($name:ident, $t:ty, $size:expr) => {
        pub struct $name {
            cell: UnsafeCell<$t>,
        }

        unsafe impl Sync for $name {}
        unsafe impl Send for $name {}

        impl $name {
            pub const fn new(x: $t) -> $name {
                $name { cell: UnsafeCell::new(x) }
            }
            fn rmw(&self, op: i32, a: $t, b: $t) -> $t {
                unsafe { rcc_thr_atomic(self.cell.get() as *mut u8, $size, op, a as u64, b as u64) as $t }
            }
            pub fn load(&self, _order: Ordering) -> $t {
                self.rmw(OP_LOAD, 0, 0)
            }
            pub fn store(&self, x: $t, _order: Ordering) {
                self.rmw(OP_STORE, x, 0);
            }
            pub fn swap(&self, x: $t, _order: Ordering) -> $t {
                self.rmw(OP_SWAP, x, 0)
            }
            pub fn compare_exchange(&self, current: $t, newv: $t, _success: Ordering, _failure: Ordering) -> Result<$t, $t> {
                let old = self.rmw(OP_CAS, current, newv);
                if old == current { Ok(old) } else { Err(old) }
            }
            pub fn compare_exchange_weak(&self, current: $t, newv: $t, success: Ordering, failure: Ordering) -> Result<$t, $t> {
                self.compare_exchange(current, newv, success, failure)
            }
            #[allow(deprecated)]
            pub fn compare_and_swap(&self, current: $t, newv: $t, _order: Ordering) -> $t {
                self.rmw(OP_CAS, current, newv)
            }
            pub fn fetch_add(&self, x: $t, _order: Ordering) -> $t {
                self.rmw(OP_ADD, x, 0)
            }
            pub fn fetch_sub(&self, x: $t, _order: Ordering) -> $t {
                self.rmw(OP_SUB, x, 0)
            }
            pub fn fetch_and(&self, x: $t, _order: Ordering) -> $t {
                self.rmw(OP_AND, x, 0)
            }
            pub fn fetch_or(&self, x: $t, _order: Ordering) -> $t {
                self.rmw(OP_OR, x, 0)
            }
            pub fn fetch_xor(&self, x: $t, _order: Ordering) -> $t {
                self.rmw(OP_XOR, x, 0)
            }
            pub fn fetch_nand(&self, x: $t, order: Ordering) -> $t {
                let mut cur = self.load(order);
                loop {
                    let old = self.rmw(OP_CAS, cur, !(cur & x));
                    if old == cur {
                        return old;
                    }
                    cur = old;
                }
            }
            pub fn fetch_max(&self, x: $t, order: Ordering) -> $t {
                let mut cur = self.load(order);
                loop {
                    let nxt = if cur > x { cur } else { x };
                    let old = self.rmw(OP_CAS, cur, nxt);
                    if old == cur {
                        return old;
                    }
                    cur = old;
                }
            }
            pub fn fetch_min(&self, x: $t, order: Ordering) -> $t {
                let mut cur = self.load(order);
                loop {
                    let nxt = if cur < x { cur } else { x };
                    let old = self.rmw(OP_CAS, cur, nxt);
                    if old == cur {
                        return old;
                    }
                    cur = old;
                }
            }
            pub fn fetch_update<F>(&self, _set_order: Ordering, fetch_order: Ordering, mut f: F) -> Result<$t, $t>
            where
                F: FnMut($t) -> Option<$t>,
            {
                let mut prev = self.load(fetch_order);
                while let Some(nxt) = f(prev) {
                    let old = self.rmw(OP_CAS, prev, nxt);
                    if old == prev {
                        return Ok(old);
                    }
                    prev = old;
                }
                Err(prev)
            }
            pub fn get_mut(&mut self) -> &mut $t {
                self.cell.get_mut()
            }
            pub fn into_inner(self) -> $t {
                self.cell.into_inner()
            }
            pub fn as_ptr(&self) -> *mut $t {
                self.cell.get()
            }
        }

        impl Default for $name {
            fn default() -> $name {
                $name::new(0)
            }
        }

        impl From<$t> for $name {
            fn from(x: $t) -> $name {
                $name::new(x)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.load(Ordering::SeqCst), f)
            }
        }
    };
}

atomic_int!(AtomicU8, u8, 1);
atomic_int!(AtomicU16, u16, 2);
atomic_int!(AtomicU32, u32, 4);
atomic_int!(AtomicU64, u64, 8);
atomic_int!(AtomicUsize, usize, 8);
atomic_int!(AtomicI8, i8, 1);
atomic_int!(AtomicI16, i16, 2);
atomic_int!(AtomicI32, i32, 4);
atomic_int!(AtomicI64, i64, 8);
atomic_int!(AtomicIsize, isize, 8);

pub struct AtomicBool {
    cell: UnsafeCell<u8>,
}

unsafe impl Sync for AtomicBool {}
unsafe impl Send for AtomicBool {}

impl AtomicBool {
    pub const fn new(b: bool) -> AtomicBool {
        AtomicBool { cell: UnsafeCell::new(b as u8) }
    }
    fn rmw(&self, op: i32, a: bool, b: bool) -> bool {
        unsafe { rcc_thr_atomic(self.cell.get(), 1, op, a as u64, b as u64) as u8 != 0 }
    }
    pub fn load(&self, _order: Ordering) -> bool {
        self.rmw(OP_LOAD, false, false)
    }
    pub fn store(&self, b: bool, _order: Ordering) {
        self.rmw(OP_STORE, b, false);
    }
    pub fn swap(&self, b: bool, _order: Ordering) -> bool {
        self.rmw(OP_SWAP, b, false)
    }
    pub fn compare_exchange(&self, current: bool, newv: bool, _success: Ordering, _failure: Ordering) -> Result<bool, bool> {
        let old = self.rmw(OP_CAS, current, newv);
        if old == current { Ok(old) } else { Err(old) }
    }
    pub fn compare_exchange_weak(&self, current: bool, newv: bool, success: Ordering, failure: Ordering) -> Result<bool, bool> {
        self.compare_exchange(current, newv, success, failure)
    }
    pub fn fetch_and(&self, b: bool, _order: Ordering) -> bool {
        self.rmw(OP_AND, b, false)
    }
    pub fn fetch_or(&self, b: bool, _order: Ordering) -> bool {
        self.rmw(OP_OR, b, false)
    }
    pub fn fetch_xor(&self, b: bool, _order: Ordering) -> bool {
        self.rmw(OP_XOR, b, false)
    }
    pub fn fetch_not(&self, order: Ordering) -> bool {
        self.fetch_xor(true, order)
    }
    pub fn fetch_nand(&self, b: bool, order: Ordering) -> bool {
        let mut cur = self.load(order);
        loop {
            let old = self.rmw(OP_CAS, cur, !(cur && b));
            if old == cur {
                return old;
            }
            cur = old;
        }
    }
    pub fn fetch_update<F>(&self, _set_order: Ordering, fetch_order: Ordering, mut f: F) -> Result<bool, bool>
    where
        F: FnMut(bool) -> Option<bool>,
    {
        let mut prev = self.load(fetch_order);
        while let Some(nxt) = f(prev) {
            let old = self.rmw(OP_CAS, prev, nxt);
            if old == prev {
                return Ok(old);
            }
            prev = old;
        }
        Err(prev)
    }
    pub fn get_mut(&mut self) -> &mut bool {
        unsafe { &mut *(self.cell.get() as *mut bool) }
    }
    pub fn into_inner(self) -> bool {
        self.cell.into_inner() != 0
    }
    pub fn as_ptr(&self) -> *mut bool {
        self.cell.get() as *mut bool
    }
}

impl Default for AtomicBool {
    fn default() -> AtomicBool {
        AtomicBool::new(false)
    }
}

impl From<bool> for AtomicBool {
    fn from(b: bool) -> AtomicBool {
        AtomicBool::new(b)
    }
}

impl fmt::Debug for AtomicBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.load(Ordering::SeqCst), f)
    }
}

pub struct AtomicPtr<T> {
    cell: UnsafeCell<*mut T>,
}

unsafe impl<T> Sync for AtomicPtr<T> {}
unsafe impl<T> Send for AtomicPtr<T> {}

impl<T> AtomicPtr<T> {
    pub const fn new(p: *mut T) -> AtomicPtr<T> {
        AtomicPtr { cell: UnsafeCell::new(p) }
    }
    fn rmw(&self, op: i32, a: *mut T, b: *mut T) -> *mut T {
        unsafe {
            rcc_thr_atomic(self.cell.get() as *mut u8, 8, op, a as usize as u64, b as usize as u64) as usize as *mut T
        }
    }
    pub fn load(&self, _order: Ordering) -> *mut T {
        self.rmw(OP_LOAD, 0usize as *mut T, 0usize as *mut T)
    }
    pub fn store(&self, p: *mut T, _order: Ordering) {
        self.rmw(OP_STORE, p, 0usize as *mut T);
    }
    pub fn swap(&self, p: *mut T, _order: Ordering) -> *mut T {
        self.rmw(OP_SWAP, p, 0usize as *mut T)
    }
    pub fn compare_exchange(&self, current: *mut T, newv: *mut T, _success: Ordering, _failure: Ordering) -> Result<*mut T, *mut T> {
        let old = self.rmw(OP_CAS, current, newv);
        if old as usize == current as usize { Ok(old) } else { Err(old) }
    }
    pub fn compare_exchange_weak(&self, current: *mut T, newv: *mut T, success: Ordering, failure: Ordering) -> Result<*mut T, *mut T> {
        self.compare_exchange(current, newv, success, failure)
    }
    pub fn get_mut(&mut self) -> &mut *mut T {
        self.cell.get_mut()
    }
    pub fn into_inner(self) -> *mut T {
        self.cell.into_inner()
    }
}

impl<T> Default for AtomicPtr<T> {
    fn default() -> AtomicPtr<T> {
        AtomicPtr::new(0usize as *mut T)
    }
}

impl<T> fmt::Debug for AtomicPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.load(Ordering::SeqCst), f)
    }
}
