//! `VecDeque<T>`: a growable ring buffer. Logical element `i` lives at
//! physical slot `(head + i) % cap`; capacity 0 holds a null pointer.

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::intrinsics;
use crate::iter::{FromIterator, FusedIterator};
use crate::ops::{Index, IndexMut, RangeBounds};
use crate::slice;

pub struct VecDeque<T> {
    buf: *mut T,
    cap: usize,
    head: usize,
    len: usize,
}

impl<T> VecDeque<T> {
    pub const fn new() -> VecDeque<T> {
        VecDeque { buf: 0usize as *mut T, cap: 0, head: 0, len: 0 }
    }
    pub fn with_capacity(capacity: usize) -> VecDeque<T> {
        VecDeque { buf: unsafe { crate::alloc::alloc_array::<T>(capacity) }, cap: capacity, head: 0, len: 0 }
    }
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn capacity(&self) -> usize {
        self.cap
    }

    #[inline]
    fn phys(&self, i: usize) -> usize {
        let p = self.head + i;
        if p >= self.cap { p - self.cap } else { p }
    }
    #[inline]
    fn ptr_at(&self, i: usize) -> *mut T {
        unsafe { self.buf.add(self.phys(i)) }
    }
    /// Move the contents into a new buffer of `new_cap` (>= len), head = 0.
    fn realloc_contiguous(&mut self, new_cap: usize) {
        unsafe {
            let nb = crate::alloc::alloc_array::<T>(new_cap);
            let first = if self.head + self.len <= self.cap { self.len } else { self.cap - self.head };
            if first > 0 {
                intrinsics::copy_nonoverlapping(self.buf.add(self.head) as *const T, nb, first);
            }
            if self.len > first {
                intrinsics::copy_nonoverlapping(self.buf as *const T, nb.add(first), self.len - first);
            }
            crate::alloc::dealloc_array(self.buf);
            self.buf = nb;
        }
        self.cap = new_cap;
        self.head = 0;
    }
    fn grow_to(&mut self, min_cap: usize) {
        if min_cap <= self.cap {
            return;
        }
        let mut new_cap = if self.cap == 0 { 4 } else { self.cap * 2 };
        if new_cap < min_cap {
            new_cap = min_cap;
        }
        self.realloc_contiguous(new_cap);
    }
    pub fn reserve(&mut self, additional: usize) {
        self.grow_to(self.len + additional);
    }
    pub fn reserve_exact(&mut self, additional: usize) {
        if self.len + additional > self.cap {
            self.realloc_contiguous(self.len + additional);
        }
    }
    pub fn shrink_to_fit(&mut self) {
        if self.cap > self.len {
            if self.len == 0 {
                unsafe { crate::alloc::dealloc_array(self.buf) };
                self.buf = 0usize as *mut T;
                self.cap = 0;
                self.head = 0;
            } else {
                self.realloc_contiguous(self.len);
            }
        }
    }

    pub fn push_back(&mut self, value: T) {
        if self.len == self.cap {
            self.grow_to(self.len + 1);
        }
        unsafe { intrinsics::write(self.ptr_at(self.len), value) };
        self.len += 1;
    }
    pub fn push_front(&mut self, value: T) {
        if self.len == self.cap {
            self.grow_to(self.len + 1);
        }
        self.head = if self.head == 0 { self.cap - 1 } else { self.head - 1 };
        unsafe { intrinsics::write(self.buf.add(self.head), value) };
        self.len += 1;
    }
    pub fn pop_back(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        unsafe { Some(intrinsics::read(self.ptr_at(self.len) as *const T)) }
    }
    pub fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let p = self.head;
        self.head = if p + 1 == self.cap { 0 } else { p + 1 };
        self.len -= 1;
        unsafe { Some(intrinsics::read(self.buf.add(p) as *const T)) }
    }
    pub fn front(&self) -> Option<&T> {
        self.get(0)
    }
    pub fn back(&self) -> Option<&T> {
        if self.len == 0 { None } else { self.get(self.len - 1) }
    }
    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.get_mut(0)
    }
    pub fn back_mut(&mut self) -> Option<&mut T> {
        if self.len == 0 {
            None
        } else {
            let i = self.len - 1;
            self.get_mut(i)
        }
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len { unsafe { Some(&*self.ptr_at(index)) } } else { None }
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.len { unsafe { Some(&mut *self.ptr_at(index)) } } else { None }
    }
    pub fn swap(&mut self, i: usize, j: usize) {
        if i >= self.len || j >= self.len {
            panic!("assertion failed: i < self.len()");
        }
        if i != j {
            unsafe {
                let a = self.ptr_at(i);
                let b = self.ptr_at(j);
                let t = intrinsics::read(a as *const T);
                intrinsics::copy_nonoverlapping(b as *const T, a, 1);
                intrinsics::write(b, t);
            }
        }
    }
    pub fn clear(&mut self) {
        self.truncate(0);
        self.head = 0;
    }
    pub fn truncate(&mut self, len: usize) {
        while self.len > len {
            self.len -= 1;
            unsafe { intrinsics::drop_in_place(self.ptr_at(self.len)) };
        }
    }
    pub fn contains(&self, x: &T) -> bool
    where
        T: PartialEq,
    {
        let (a, b) = self.as_slices();
        a.contains(x) || b.contains(x)
    }
    pub fn as_slices(&self) -> (&[T], &[T]) {
        unsafe {
            if self.head + self.len <= self.cap {
                (slice::from_raw_parts(self.buf.add(self.head) as *const T, self.len), slice::from_raw_parts(self.buf as *const T, 0))
            } else {
                let first = self.cap - self.head;
                (
                    slice::from_raw_parts(self.buf.add(self.head) as *const T, first),
                    slice::from_raw_parts(self.buf as *const T, self.len - first),
                )
            }
        }
    }
    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        unsafe {
            if self.head + self.len <= self.cap {
                (slice::from_raw_parts_mut(self.buf.add(self.head), self.len), slice::from_raw_parts_mut(self.buf, 0))
            } else {
                let first = self.cap - self.head;
                (slice::from_raw_parts_mut(self.buf.add(self.head), first), slice::from_raw_parts_mut(self.buf, self.len - first))
            }
        }
    }
    pub fn make_contiguous(&mut self) -> &mut [T] {
        if self.head + self.len > self.cap {
            let c = self.cap;
            self.realloc_contiguous(c);
        }
        unsafe { slice::from_raw_parts_mut(self.buf.add(self.head), self.len) }
    }
    pub fn iter(&self) -> Iter<'_, T> {
        let (a, b) = self.as_slices();
        Iter { a: a.iter(), b: b.iter() }
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        let (a, b) = self.as_mut_slices();
        IterMut { a: a.iter_mut(), b: b.iter_mut() }
    }
    pub fn range<R: RangeBounds<usize>>(&self, range: R) -> Iter<'_, T> {
        let (start, end) = crate::ops::range_to_bounds(&range, self.len);
        let mut it = self.iter();
        let mut i = 0;
        while i < start {
            it.next();
            i += 1;
        }
        let mut j = self.len;
        while j > end {
            it.next_back();
            j -= 1;
        }
        it
    }
    pub fn insert(&mut self, index: usize, value: T) {
        if index > self.len {
            panic!("index out of bounds");
        }
        if self.len == self.cap {
            self.grow_to(self.len + 1);
        }
        // shift [index, len) back by one
        let mut i = self.len;
        while i > index {
            unsafe { intrinsics::copy_nonoverlapping(self.ptr_at(i - 1) as *const T, self.ptr_at(i), 1) };
            i -= 1;
        }
        unsafe { intrinsics::write(self.ptr_at(index), value) };
        self.len += 1;
    }
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.len {
            return None;
        }
        let v = unsafe { intrinsics::read(self.ptr_at(index) as *const T) };
        let mut i = index;
        while i + 1 < self.len {
            unsafe { intrinsics::copy_nonoverlapping(self.ptr_at(i + 1) as *const T, self.ptr_at(i), 1) };
            i += 1;
        }
        self.len -= 1;
        Some(v)
    }
    pub fn swap_remove_back(&mut self, index: usize) -> Option<T> {
        let length = self.len;
        if index < length && index != length - 1 {
            self.swap(index, length - 1);
        } else if index >= length {
            return None;
        }
        self.pop_back()
    }
    pub fn swap_remove_front(&mut self, index: usize) -> Option<T> {
        let length = self.len;
        if index < length && index != 0 {
            self.swap(index, 0);
        } else if index >= length {
            return None;
        }
        self.pop_front()
    }
    pub fn rotate_left(&mut self, n: usize) {
        if n > self.len {
            panic!("assertion failed: n <= self.len()");
        }
        self.make_contiguous().rotate_left(n);
    }
    pub fn rotate_right(&mut self, n: usize) {
        if n > self.len {
            panic!("assertion failed: n <= self.len()");
        }
        self.make_contiguous().rotate_right(n);
    }
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut f: F) {
        self.retain_mut(|x| f(x))
    }
    pub fn retain_mut<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
        self.make_contiguous();
        let len = self.len;
        let base = unsafe { self.buf.add(self.head) };
        let mut kept = 0;
        let mut i = 0;
        self.len = 0;
        while i < len {
            unsafe {
                let p = base.add(i);
                if f(&mut *p) {
                    if kept != i {
                        intrinsics::copy_nonoverlapping(p as *const T, base.add(kept), 1);
                    }
                    kept += 1;
                } else {
                    intrinsics::drop_in_place(p);
                }
            }
            i += 1;
        }
        self.len = kept;
    }
    pub fn resize_with<F: FnMut() -> T>(&mut self, new_len: usize, mut f: F) {
        if new_len <= self.len {
            self.truncate(new_len);
        } else {
            self.reserve(new_len - self.len);
            while self.len < new_len {
                self.push_back(f());
            }
        }
    }
    pub fn resize(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
        if new_len <= self.len {
            self.truncate(new_len);
        } else {
            self.reserve(new_len - self.len);
            while self.len + 1 < new_len {
                self.push_back(value.clone());
            }
            self.push_back(value);
        }
    }
    pub fn split_off(&mut self, at: usize) -> VecDeque<T> {
        if at > self.len {
            panic!("`at` out of bounds");
        }
        let n = self.len - at;
        let mut other = VecDeque::with_capacity(n);
        let mut i = 0;
        while i < n {
            unsafe { intrinsics::write(other.buf.add(i), intrinsics::read(self.ptr_at(at + i) as *const T)) };
            i += 1;
        }
        other.len = n;
        self.len = at;
        other
    }
    pub fn append(&mut self, other: &mut VecDeque<T>) {
        self.reserve(other.len);
        while let Some(x) = other.pop_front() {
            self.push_back(x);
        }
        other.head = 0;
    }
    /// Remove a range and return its elements (collected eagerly).
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<'_, T> {
        let (start, end) = crate::ops::range_to_bounds(&range, self.len);
        let n = end - start;
        let mut removed: Vec<T> = Vec::with_capacity(n);
        let mut i = start;
        while i < end {
            unsafe { removed.push(intrinsics::read(self.ptr_at(i) as *const T)) };
            i += 1;
        }
        if start < self.len - end {
            // fewer elements before the gap: shift the front part back
            let mut k = start;
            while k > 0 {
                k -= 1;
                unsafe { intrinsics::copy_nonoverlapping(self.ptr_at(k) as *const T, self.ptr_at(k + n), 1) };
            }
            self.head = self.phys(n);
            if self.len == n {
                self.head = 0;
            }
        } else {
            let mut k = end;
            while k < self.len {
                unsafe { intrinsics::copy_nonoverlapping(self.ptr_at(k) as *const T, self.ptr_at(k - n), 1) };
                k += 1;
            }
        }
        self.len -= n;
        Drain { inner: removed.into_iter(), marker: crate::marker::PhantomData }
    }
    pub fn binary_search_by<F: FnMut(&T) -> Ordering>(&self, mut f: F) -> Result<usize, usize> {
        let mut lo = 0usize;
        let mut hi = self.len;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match f(&self[mid]) {
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid,
                Ordering::Equal => return Ok(mid),
            }
        }
        Err(lo)
    }
    pub fn binary_search(&self, x: &T) -> Result<usize, usize>
    where
        T: Ord,
    {
        self.binary_search_by(|e| e.cmp(x))
    }
    pub fn binary_search_by_key<B: Ord, F: FnMut(&T) -> B>(&self, b: &B, mut f: F) -> Result<usize, usize> {
        self.binary_search_by(|k| f(k).cmp(b))
    }
    pub fn partition_point<P: FnMut(&T) -> bool>(&self, mut pred: P) -> usize {
        self.binary_search_by(|x| if pred(x) { Ordering::Less } else { Ordering::Greater }).unwrap_or_else(|i| i)
    }
}

impl<T> Drop for VecDeque<T> {
    fn drop(&mut self) {
        let (a, b) = self.as_mut_slices();
        unsafe {
            intrinsics::drop_in_place(a as *mut [T]);
            intrinsics::drop_in_place(b as *mut [T]);
            crate::alloc::dealloc_array(self.buf);
        }
    }
}

impl<T> Index<usize> for VecDeque<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        match self.get(index) {
            Some(x) => x,
            None => panic!("Out of bounds access"),
        }
    }
}
impl<T> IndexMut<usize> for VecDeque<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        match self.get_mut(index) {
            Some(x) => x,
            None => panic!("Out of bounds access"),
        }
    }
}

impl<T: Clone> Clone for VecDeque<T> {
    fn clone(&self) -> VecDeque<T> {
        let mut d = VecDeque::with_capacity(self.len);
        for x in self.iter() {
            d.push_back(x.clone());
        }
        d
    }
}
impl<T> Default for VecDeque<T> {
    fn default() -> VecDeque<T> {
        VecDeque::new()
    }
}
impl<T: fmt::Debug> fmt::Debug for VecDeque<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
impl<T: PartialEq> PartialEq for VecDeque<T> {
    fn eq(&self, other: &VecDeque<T>) -> bool {
        if self.len != other.len {
            return false;
        }
        let mut i = 0;
        while i < self.len {
            if !(self[i] == other[i]) {
                return false;
            }
            i += 1;
        }
        true
    }
}
impl<T: Eq> Eq for VecDeque<T> {}
impl<T: PartialEq<U>, U> PartialEq<Vec<U>> for VecDeque<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        if self.len != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len {
            if !(self[i] == other[i]) {
                return false;
            }
            i += 1;
        }
        true
    }
}
impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for VecDeque<T> {
    fn eq(&self, other: &[U; N]) -> bool {
        if self.len != N {
            return false;
        }
        let mut i = 0;
        while i < N {
            if !(self[i] == other[i]) {
                return false;
            }
            i += 1;
        }
        true
    }
}
impl<T: PartialEq<U>, U> PartialEq<&[U]> for VecDeque<T> {
    fn eq(&self, other: &&[U]) -> bool {
        if self.len != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len {
            if !(self[i] == other[i]) {
                return false;
            }
            i += 1;
        }
        true
    }
}
impl<T: PartialOrd> PartialOrd for VecDeque<T> {
    fn partial_cmp(&self, other: &VecDeque<T>) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}
impl<T: Ord> Ord for VecDeque<T> {
    fn cmp(&self, other: &VecDeque<T>) -> Ordering {
        self.iter().cmp(other.iter())
    }
}
impl<T: Hash> Hash for VecDeque<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_length_prefix(self.len);
        for x in self.iter() {
            x.hash(state);
        }
    }
}

impl<T> Extend<T> for VecDeque<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let it = iter.into_iter();
        let (lo, _) = it.size_hint();
        self.reserve(lo);
        for x in it {
            self.push_back(x);
        }
    }
}
impl<'a, T: 'a + Copy> Extend<&'a T> for VecDeque<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        for x in iter {
            self.push_back(*x);
        }
    }
}
impl<T> FromIterator<T> for VecDeque<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> VecDeque<T> {
        let mut d = VecDeque::new();
        d.extend(iter);
        d
    }
}
impl<T> From<Vec<T>> for VecDeque<T> {
    fn from(v: Vec<T>) -> VecDeque<T> {
        let mut v = v;
        let len = v.len();
        let cap = v.capacity();
        let ptr = v.as_mut_ptr();
        intrinsics_forget(v);
        VecDeque { buf: ptr, cap, head: 0, len }
    }
}
impl<T> From<VecDeque<T>> for Vec<T> {
    fn from(d: VecDeque<T>) -> Vec<T> {
        let mut d = d;
        d.make_contiguous();
        let mut v = Vec::with_capacity(d.len);
        unsafe {
            intrinsics::copy_nonoverlapping(d.buf.add(d.head) as *const T, v.as_mut_ptr(), d.len);
            v.set_len(d.len);
        }
        d.len = 0;
        v
    }
}
impl<T, const N: usize> From<[T; N]> for VecDeque<T> {
    fn from(arr: [T; N]) -> VecDeque<T> {
        VecDeque::from(crate::vec::from_array(arr))
    }
}

#[inline]
fn intrinsics_forget<T>(x: T) {
    unsafe { intrinsics::forget(x) }
}

impl<'a, T> IntoIterator for &'a VecDeque<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}
impl<'a, T> IntoIterator for &'a mut VecDeque<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}
impl<T> IntoIterator for VecDeque<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { inner: self }
    }
}

// ---- iterators -------------------------------------------------------------

pub struct Iter<'a, T> {
    a: slice::Iter<'a, T>,
    b: slice::Iter<'a, T>,
}
impl<'a, T> Clone for Iter<'a, T> {
    fn clone(&self) -> Iter<'a, T> {
        Iter { a: self.a.clone(), b: self.b.clone() }
    }
}
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        match self.a.next() {
            Some(x) => Some(x),
            None => self.b.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.a.len() + self.b.len();
        (n, Some(n))
    }
    fn count(self) -> usize {
        self.a.len() + self.b.len()
    }
}
impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a T> {
        match self.b.next_back() {
            Some(x) => Some(x),
            None => self.a.next_back(),
        }
    }
}
impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.a.len() + self.b.len()
    }
}
impl<'a, T> FusedIterator for Iter<'a, T> {}
impl<'a, T: fmt::Debug> fmt::Debug for Iter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").field(&self.a.as_slice()).field(&self.b.as_slice()).finish()
    }
}

pub struct IterMut<'a, T> {
    a: slice::IterMut<'a, T>,
    b: slice::IterMut<'a, T>,
}
impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    #[inline]
    fn next(&mut self) -> Option<&'a mut T> {
        match self.a.next() {
            Some(x) => Some(x),
            None => self.b.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.a.len() + self.b.len();
        (n, Some(n))
    }
}
impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut T> {
        match self.b.next_back() {
            Some(x) => Some(x),
            None => self.a.next_back(),
        }
    }
}
impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.a.len() + self.b.len()
    }
}
impl<'a, T> FusedIterator for IterMut<'a, T> {}

pub struct IntoIter<T> {
    inner: VecDeque<T>,
}
impl<T> Iterator for IntoIter<T> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        self.inner.pop_front()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.inner.len, Some(self.inner.len))
    }
}
impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.inner.pop_back()
    }
}
impl<T> ExactSizeIterator for IntoIter<T> {}
impl<T> FusedIterator for IntoIter<T> {}
impl<T: Clone> Clone for IntoIter<T> {
    fn clone(&self) -> IntoIter<T> {
        IntoIter { inner: self.inner.clone() }
    }
}

pub struct Drain<'a, T> {
    inner: crate::vec::IntoIter<T>,
    marker: crate::marker::PhantomData<&'a mut T>,
}
impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<'a, T> DoubleEndedIterator for Drain<'a, T> {
    fn next_back(&mut self) -> Option<T> {
        self.inner.next_back()
    }
}
impl<'a, T> ExactSizeIterator for Drain<'a, T> {}
impl<'a, T> FusedIterator for Drain<'a, T> {}
