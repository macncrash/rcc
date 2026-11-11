//! `Vec<T>`: pointer, capacity, length. An empty vector with capacity 0
//! holds a null pointer (never dereferenced).

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::Hash;
use crate::intrinsics;
use crate::ops::{Deref, DerefMut, RangeBounds};
use crate::slice;

pub struct Vec<T> {
    ptr: *mut T,
    cap: usize,
    len: usize,
}

/// `vec![x; n]`
pub fn from_elem<T: Clone>(elem: T, n: usize) -> Vec<T> {
    let mut v = Vec::with_capacity(n);
    if n > 0 {
        let mut i = 1;
        while i < n {
            v.push(elem.clone());
            i += 1;
        }
        v.push(elem);
    }
    v
}

/// `vec![a, b, c]`
pub fn from_array<T, const N: usize>(a: [T; N]) -> Vec<T> {
    let mut v = Vec::with_capacity(N);
    unsafe {
        intrinsics::copy_nonoverlapping(&a as *const [T; N] as *const T, v.ptr, N);
        v.len = N;
        intrinsics::forget(a);
    }
    v
}

impl<T> Vec<T> {
    pub const fn new() -> Vec<T> {
        Vec { ptr: 0usize as *mut T, cap: 0, len: 0 }
    }
    pub fn with_capacity(capacity: usize) -> Vec<T> {
        Vec { ptr: unsafe { crate::alloc::alloc_array::<T>(capacity) }, cap: capacity, len: 0 }
    }
    pub unsafe fn from_raw_parts(ptr: *mut T, length: usize, capacity: usize) -> Vec<T> {
        Vec { ptr, cap: capacity, len: length }
    }
    pub const fn len(&self) -> usize {
        self.len
    }
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn capacity(&self) -> usize {
        self.cap
    }
    pub fn as_ptr(&self) -> *const T {
        self.ptr as *const T
    }
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr
    }
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr as *const T, self.len) }
    }
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.len = new_len;
    }
    fn grow_to(&mut self, min_cap: usize) {
        if min_cap <= self.cap {
            return;
        }
        let mut new_cap = if self.cap == 0 { 4 } else { self.cap * 2 };
        if new_cap < min_cap {
            new_cap = min_cap;
        }
        self.ptr = unsafe { crate::alloc::realloc_array(self.ptr, new_cap) };
        self.cap = new_cap;
    }
    pub fn reserve(&mut self, additional: usize) {
        self.grow_to(self.len + additional);
    }
    pub fn reserve_exact(&mut self, additional: usize) {
        let need = self.len + additional;
        if need > self.cap {
            self.ptr = unsafe { crate::alloc::realloc_array(self.ptr, need) };
            self.cap = need;
        }
    }
    pub fn shrink_to_fit(&mut self) {
        if self.len < self.cap && self.len > 0 {
            self.ptr = unsafe { crate::alloc::realloc_array(self.ptr, self.len) };
            self.cap = self.len;
        }
    }
    pub fn shrink_to(&mut self, _min_capacity: usize) {}
    pub fn push(&mut self, value: T) {
        if self.len == self.cap {
            self.grow_to(self.len + 1);
        }
        unsafe { intrinsics::write(self.ptr.add(self.len), value) };
        self.len += 1;
    }
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(intrinsics::read(self.ptr.add(self.len) as *const T)) }
        }
    }
    pub fn pop_if<F: FnOnce(&mut T) -> bool>(&mut self, f: F) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let last = unsafe { &mut *self.ptr.add(self.len - 1) };
        if f(last) { self.pop() } else { None }
    }
    pub fn clear(&mut self) {
        self.truncate(0)
    }
    pub fn truncate(&mut self, len: usize) {
        if len >= self.len {
            return;
        }
        let old = self.len;
        self.len = len;
        unsafe { intrinsics::drop_in_place(crate::ptr::slice_from_raw_parts_mut(self.ptr.add(len), old - len)) }
    }
    pub fn insert(&mut self, index: usize, element: T) {
        let len = self.len;
        if index > len {
            panic!("insertion index (is {}) should be <= len (is {})", index, len);
        }
        if len == self.cap {
            self.grow_to(len + 1);
        }
        unsafe {
            let p = self.ptr.add(index);
            intrinsics::copy(p as *const T, p.add(1), len - index);
            intrinsics::write(p, element);
        }
        self.len = len + 1;
    }
    pub fn remove(&mut self, index: usize) -> T {
        let len = self.len;
        if index >= len {
            panic!("removal index (is {}) should be < len (is {})", index, len);
        }
        unsafe {
            let p = self.ptr.add(index);
            let v = intrinsics::read(p as *const T);
            intrinsics::copy(p.add(1) as *const T, p, len - index - 1);
            self.len = len - 1;
            v
        }
    }
    pub fn swap_remove(&mut self, index: usize) -> T {
        let len = self.len;
        if index >= len {
            panic!("swap_remove index (is {}) should be < len (is {})", index, len);
        }
        unsafe {
            let p = self.ptr.add(index);
            let v = intrinsics::read(p as *const T);
            intrinsics::copy(self.ptr.add(len - 1) as *const T, p, 1);
            self.len = len - 1;
            v
        }
    }
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut f: F) {
        self.retain_mut(|x| f(x))
    }
    pub fn retain_mut<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
        let len = self.len;
        let mut kept = 0;
        let mut i = 0;
        // len is restored at the end; elements are moved down in place
        self.len = 0;
        while i < len {
            unsafe {
                let p = self.ptr.add(i);
                if f(&mut *p) {
                    if kept != i {
                        intrinsics::copy_nonoverlapping(p as *const T, self.ptr.add(kept), 1);
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
    pub fn dedup_by<F: FnMut(&mut T, &mut T) -> bool>(&mut self, mut same_bucket: F) {
        let len = self.len;
        if len < 2 {
            return;
        }
        let mut kept = 1;
        let mut i = 1;
        self.len = 0;
        while i < len {
            unsafe {
                let p = self.ptr.add(i);
                let prev = self.ptr.add(kept - 1);
                if same_bucket(&mut *p, &mut *prev) {
                    intrinsics::drop_in_place(p);
                } else {
                    if kept != i {
                        intrinsics::copy_nonoverlapping(p as *const T, self.ptr.add(kept), 1);
                    }
                    kept += 1;
                }
            }
            i += 1;
        }
        self.len = kept;
    }
    pub fn dedup_by_key<K: PartialEq, F: FnMut(&mut T) -> K>(&mut self, mut key: F) {
        self.dedup_by(|a, b| key(a) == key(b))
    }
    pub fn dedup(&mut self)
    where
        T: PartialEq,
    {
        self.dedup_by(|a, b| *a == *b)
    }
    pub fn append(&mut self, other: &mut Vec<T>) {
        let n = other.len;
        self.reserve(n);
        unsafe {
            intrinsics::copy_nonoverlapping(other.ptr as *const T, self.ptr.add(self.len), n);
        }
        self.len += n;
        other.len = 0;
    }
    pub fn extend_from_slice(&mut self, other: &[T])
    where
        T: Clone,
    {
        self.reserve(other.len());
        let mut i = 0;
        while i < other.len() {
            self.push(other[i].clone());
            i += 1;
        }
    }
    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, src: R)
    where
        T: Clone,
    {
        let (start, end) = crate::ops::range_to_bounds(&src, self.len);
        self.reserve(end - start);
        let mut i = start;
        while i < end {
            let v = self[i].clone();
            self.push(v);
            i += 1;
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
                self.push(value.clone());
            }
            self.push(value);
        }
    }
    pub fn resize_with<F: FnMut() -> T>(&mut self, new_len: usize, mut f: F) {
        if new_len <= self.len {
            self.truncate(new_len);
        } else {
            self.reserve(new_len - self.len);
            while self.len < new_len {
                self.push(f());
            }
        }
    }
    pub fn split_off(&mut self, at: usize) -> Vec<T> {
        if at > self.len {
            panic!("`at` split index (is {}) should be <= len (is {})", at, self.len);
        }
        let n = self.len - at;
        let mut other = Vec::with_capacity(n);
        unsafe {
            intrinsics::copy_nonoverlapping(self.ptr.add(at) as *const T, other.ptr, n);
        }
        other.len = n;
        self.len = at;
        other
    }
    /// Remove a range and return its elements (collected eagerly).
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain<T> {
        let (start, end) = crate::ops::range_to_bounds(&range, self.len);
        let n = end - start;
        let mut removed = Vec::with_capacity(n);
        unsafe {
            intrinsics::copy_nonoverlapping(self.ptr.add(start) as *const T, removed.ptr, n);
            removed.len = n;
            intrinsics::copy(self.ptr.add(end) as *const T, self.ptr.add(start), self.len - end);
        }
        self.len -= n;
        Drain { iter: removed.into_iter() }
    }
    /// Replace a range by the items of `replace_with`; returns the removed
    /// elements.
    pub fn splice<R: RangeBounds<usize>, I: IntoIterator<Item = T>>(&mut self, range: R, replace_with: I) -> Drain<T> {
        let (start, end) = crate::ops::range_to_bounds(&range, self.len);
        let mut tail = self.split_off(end);
        let removed = self.split_off(start);
        self.extend(replace_with);
        self.append(&mut tail);
        Drain { iter: removed.into_iter() }
    }
    pub fn into_boxed_slice(mut self) -> crate::boxed::Box<[T]> {
        self.shrink_to_fit();
        unsafe {
            let p = crate::ptr::slice_from_raw_parts_mut(self.ptr, self.len);
            intrinsics::forget(self);
            crate::boxed::Box::from_raw(p)
        }
    }
    pub fn leak<'a>(self) -> &'a mut [T] {
        unsafe {
            let s = slice::from_raw_parts_mut(self.ptr, self.len);
            intrinsics::forget(self);
            s
        }
    }
    pub fn spare_capacity(&self) -> usize {
        self.cap - self.len
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        unsafe {
            intrinsics::drop_in_place(crate::ptr::slice_from_raw_parts_mut(self.ptr, self.len));
            crate::alloc::dealloc_array(self.ptr);
        }
    }
}

impl<T> Deref for Vec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr as *const T, self.len) }
    }
}
impl<T> DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T: Clone> Clone for Vec<T> {
    fn clone(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }
}
impl<T> Default for Vec<T> {
    fn default() -> Vec<T> {
        Vec::new()
    }
}
impl<T: fmt::Debug> fmt::Debug for Vec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_slice(), f)
    }
}
impl<T: crate::hash::Hash> crate::hash::Hash for Vec<T> {
    fn hash<H: crate::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state)
    }
}

impl<T: PartialEq<U>, U> PartialEq<Vec<U>> for Vec<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl<T: PartialEq<U>, U> PartialEq<[U]> for Vec<T> {
    fn eq(&self, other: &[U]) -> bool {
        self.as_slice() == other
    }
}
impl<T: PartialEq<U>, U> PartialEq<&[U]> for Vec<T> {
    fn eq(&self, other: &&[U]) -> bool {
        self.as_slice() == *other
    }
}
impl<T: PartialEq<U>, U> PartialEq<&mut [U]> for Vec<T> {
    fn eq(&self, other: &&mut [U]) -> bool {
        self.as_slice() == &**other
    }
}
impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for Vec<T> {
    fn eq(&self, other: &[U; N]) -> bool {
        let o: &[U] = other;
        self.as_slice() == o
    }
}
impl<T: PartialEq<U>, U, const N: usize> PartialEq<&[U; N]> for Vec<T> {
    fn eq(&self, other: &&[U; N]) -> bool {
        let o: &[U] = *other;
        self.as_slice() == o
    }
}
impl<T: PartialEq<U>, U> PartialEq<Vec<U>> for [T] {
    fn eq(&self, other: &Vec<U>) -> bool {
        self == other.as_slice()
    }
}
impl<T: PartialEq<U>, U> PartialEq<Vec<U>> for &[T] {
    fn eq(&self, other: &Vec<U>) -> bool {
        *self == other.as_slice()
    }
}
impl<T: Eq> Eq for Vec<T> {}
impl<T: PartialOrd> PartialOrd for Vec<T> {
    fn partial_cmp(&self, other: &Vec<T>) -> Option<Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}
impl<T: Ord> Ord for Vec<T> {
    fn cmp(&self, other: &Vec<T>) -> Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl<T> AsRef<[T]> for Vec<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T> AsRef<Vec<T>> for Vec<T> {
    fn as_ref(&self) -> &Vec<T> {
        self
    }
}
impl<T> AsMut<[T]> for Vec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}
impl<T> crate::borrow::Borrow<[T]> for Vec<T> {
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: Clone> From<&[T]> for Vec<T> {
    fn from(s: &[T]) -> Vec<T> {
        s.to_vec()
    }
}
impl<T: Clone> From<&mut [T]> for Vec<T> {
    fn from(s: &mut [T]) -> Vec<T> {
        s.to_vec()
    }
}
impl<T, const N: usize> From<[T; N]> for Vec<T> {
    fn from(a: [T; N]) -> Vec<T> {
        from_array(a)
    }
}
impl<T: Clone, const N: usize> From<&[T; N]> for Vec<T> {
    fn from(a: &[T; N]) -> Vec<T> {
        let s: &[T] = a;
        s.to_vec()
    }
}
impl From<&str> for Vec<u8> {
    fn from(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }
}
impl<T> From<crate::boxed::Box<[T]>> for Vec<T> {
    fn from(b: crate::boxed::Box<[T]>) -> Vec<T> {
        b.into_vec()
    }
}
impl<T> From<Vec<T>> for crate::boxed::Box<[T]> {
    fn from(v: Vec<T>) -> crate::boxed::Box<[T]> {
        v.into_boxed_slice()
    }
}

impl<T> Extend<T> for Vec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        let (lo, _) = it.size_hint();
        self.reserve(lo);
        while let Some(x) = it.next() {
            self.push(x);
        }
    }
}
impl<'a, T: Copy + 'a> Extend<&'a T> for Vec<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        let mut it = iter.into_iter();
        while let Some(x) = it.next() {
            self.push(*x);
        }
    }
}
impl<T> FromIterator<T> for Vec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Vec<T> {
        let mut v = Vec::new();
        v.extend(iter);
        v
    }
}

/// By-value iterator: owns the buffer; [pos, end) are the live elements.
pub struct IntoIter<T> {
    buf: *mut T,
    pos: usize,
    end: usize,
}

impl<T> IntoIter<T> {
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.buf.add(self.pos) as *const T, self.end - self.pos) }
    }
}
impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.pos == self.end {
            None
        } else {
            let i = self.pos;
            self.pos += 1;
            unsafe { Some(intrinsics::read(self.buf.add(i) as *const T)) }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.end - self.pos, Some(self.end - self.pos))
    }
}
impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        if self.pos == self.end {
            None
        } else {
            self.end -= 1;
            unsafe { Some(intrinsics::read(self.buf.add(self.end) as *const T)) }
        }
    }
}
impl<T> ExactSizeIterator for IntoIter<T> {}
impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        unsafe {
            intrinsics::drop_in_place(crate::ptr::slice_from_raw_parts_mut(self.buf.add(self.pos), self.end - self.pos));
            crate::alloc::dealloc_array(self.buf);
        }
    }
}
impl<T: Clone> Clone for IntoIter<T> {
    fn clone(&self) -> IntoIter<T> {
        self.as_slice().to_vec().into_iter()
    }
}

impl<T> IntoIterator for Vec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        let it = IntoIter { buf: self.ptr, pos: 0, end: self.len };
        unsafe { intrinsics::forget(self) };
        it
    }
}
impl<'a, T> IntoIterator for &'a Vec<T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;
    fn into_iter(self) -> slice::Iter<'a, T> {
        self.as_slice().iter()
    }
}
impl<'a, T> IntoIterator for &'a mut Vec<T> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;
    fn into_iter(self) -> slice::IterMut<'a, T> {
        self.as_mut_slice().iter_mut()
    }
}

pub struct Drain<T> {
    iter: IntoIter<T>,
}
impl<T> Iterator for Drain<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<T> DoubleEndedIterator for Drain<T> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}
impl<T> ExactSizeIterator for Drain<T> {}
