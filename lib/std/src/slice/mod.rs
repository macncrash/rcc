//! Slices.

mod index;
mod iter;
mod sort;
mod join;

pub use self::index::SliceIndex;
pub use self::iter::{ChunkBy, Chunks, ChunksExact, ChunksMut, Iter, IterMut, RChunks, Split, Windows};
pub use self::join::{Concat, Join};

use crate::cmp::Ordering;
use crate::intrinsics;
use crate::vec::Vec;

pub unsafe fn from_raw_parts<'a, T>(data: *const T, len: usize) -> &'a [T] {
    &*intrinsics::slice_from_raw_parts(data, len)
}
pub unsafe fn from_raw_parts_mut<'a, T>(data: *mut T, len: usize) -> &'a mut [T] {
    &mut *(intrinsics::slice_from_raw_parts(data as *const T, len) as *mut [T])
}
pub fn from_ref<T>(s: &T) -> &[T] {
    unsafe { from_raw_parts(s as *const T, 1) }
}
pub fn from_mut<T>(s: &mut T) -> &mut [T] {
    unsafe { from_raw_parts_mut(s as *mut T, 1) }
}

fn index_fail(index: usize, len: usize) -> ! {
    panic!("index out of bounds: the len is {} but the index is {}", len, index)
}

impl<T> [T] {
    pub const fn len(&self) -> usize {
        unsafe { intrinsics::slice_len(self as *const [T]) }
    }
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub const fn as_ptr(&self) -> *const T {
        self as *const [T] as *const T
    }
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self as *mut [T] as *mut T
    }
    pub fn first(&self) -> Option<&T> {
        if self.len() == 0 { None } else { Some(&self[0]) }
    }
    pub fn first_mut(&mut self) -> Option<&mut T> {
        if self.len() == 0 { None } else { Some(&mut self[0]) }
    }
    pub fn last(&self) -> Option<&T> {
        let n = self.len();
        if n == 0 { None } else { Some(&self[n - 1]) }
    }
    pub fn last_mut(&mut self) -> Option<&mut T> {
        let n = self.len();
        if n == 0 { None } else { Some(&mut self[n - 1]) }
    }
    pub fn split_first(&self) -> Option<(&T, &[T])> {
        if self.len() == 0 { None } else { Some((&self[0], &self[1..])) }
    }
    pub fn split_last(&self) -> Option<(&T, &[T])> {
        let n = self.len();
        if n == 0 { None } else { Some((&self[n - 1], &self[..n - 1])) }
    }
    pub fn split_first_mut(&mut self) -> Option<(&mut T, &mut [T])> {
        let n = self.len();
        if n == 0 {
            return None;
        }
        let p = self.as_mut_ptr();
        unsafe { Some((&mut *p, from_raw_parts_mut(p.add(1), n - 1))) }
    }
    pub fn split_last_mut(&mut self) -> Option<(&mut T, &mut [T])> {
        let n = self.len();
        if n == 0 {
            return None;
        }
        let p = self.as_mut_ptr();
        unsafe { Some((&mut *p.add(n - 1), from_raw_parts_mut(p, n - 1))) }
    }
    pub fn get<I: SliceIndex<[T]>>(&self, index: I) -> Option<&I::Output> {
        index.get(self)
    }
    pub fn get_mut<I: SliceIndex<[T]>>(&mut self, index: I) -> Option<&mut I::Output> {
        index.get_mut(self)
    }
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        &*self.as_ptr().add(index)
    }
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        &mut *self.as_mut_ptr().add(index)
    }
    pub fn iter(&self) -> Iter<'_, T> {
        iter::iter_new(self)
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        iter::iter_mut_new(self)
    }
    pub fn swap(&mut self, a: usize, b: usize) {
        let n = self.len();
        if a >= n {
            index_fail(a, n);
        }
        if b >= n {
            index_fail(b, n);
        }
        unsafe {
            let p = self.as_mut_ptr();
            crate::ptr::swap(p.add(a), p.add(b));
        }
    }
    pub fn reverse(&mut self) {
        let n = self.len();
        let mut i = 0;
        while i < n / 2 {
            self.swap(i, n - 1 - i);
            i += 1;
        }
    }
    pub fn split_at(&self, mid: usize) -> (&[T], &[T]) {
        (&self[..mid], &self[mid..])
    }
    pub fn split_at_checked(&self, mid: usize) -> Option<(&[T], &[T])> {
        if mid <= self.len() { Some((&self[..mid], &self[mid..])) } else { None }
    }
    pub fn split_at_mut(&mut self, mid: usize) -> (&mut [T], &mut [T]) {
        let n = self.len();
        if mid > n {
            panic!("mid > len");
        }
        let p = self.as_mut_ptr();
        unsafe { (from_raw_parts_mut(p, mid), from_raw_parts_mut(p.add(mid), n - mid)) }
    }
    pub fn contains(&self, x: &T) -> bool
    where
        T: PartialEq,
    {
        let mut i = 0;
        while i < self.len() {
            if self[i] == *x {
                return true;
            }
            i += 1;
        }
        false
    }
    pub fn starts_with(&self, needle: &[T]) -> bool
    where
        T: PartialEq,
    {
        let n = needle.len();
        self.len() >= n && self[..n] == *needle
    }
    pub fn ends_with(&self, needle: &[T]) -> bool
    where
        T: PartialEq,
    {
        let (m, n) = (self.len(), needle.len());
        m >= n && self[m - n..] == *needle
    }
    pub fn strip_prefix(&self, prefix: &[T]) -> Option<&[T]>
    where
        T: PartialEq,
    {
        if self.starts_with(prefix) { Some(&self[prefix.len()..]) } else { None }
    }
    pub fn strip_suffix(&self, suffix: &[T]) -> Option<&[T]>
    where
        T: PartialEq,
    {
        if self.ends_with(suffix) { Some(&self[..self.len() - suffix.len()]) } else { None }
    }
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        let mut v = Vec::with_capacity(self.len());
        let mut i = 0;
        while i < self.len() {
            v.push(self[i].clone());
            i += 1;
        }
        v
    }
    pub fn into_vec(self: crate::boxed::Box<[T]>) -> Vec<T> {
        let len = self.len();
        let p = crate::boxed::Box::into_raw(self) as *mut T;
        unsafe { Vec::from_raw_parts(p, len, len) }
    }
    pub fn repeat(&self, n: usize) -> Vec<T>
    where
        T: Copy,
    {
        let mut v = Vec::with_capacity(self.len() * n);
        let mut i = 0;
        while i < n {
            v.extend_from_slice(self);
            i += 1;
        }
        v
    }
    pub fn fill(&mut self, value: T)
    where
        T: Clone,
    {
        let mut i = 0;
        while i < self.len() {
            self[i] = value.clone();
            i += 1;
        }
    }
    pub fn fill_with<F: FnMut() -> T>(&mut self, mut f: F) {
        let mut i = 0;
        while i < self.len() {
            self[i] = f();
            i += 1;
        }
    }
    pub fn copy_from_slice(&mut self, src: &[T])
    where
        T: Copy,
    {
        if self.len() != src.len() {
            panic!("source slice length ({}) does not match destination slice length ({})", src.len(), self.len());
        }
        unsafe { intrinsics::copy(src.as_ptr(), self.as_mut_ptr(), src.len()) }
    }
    pub fn clone_from_slice(&mut self, src: &[T])
    where
        T: Clone,
    {
        if self.len() != src.len() {
            panic!("destination and source slices have different lengths");
        }
        let mut i = 0;
        while i < src.len() {
            self[i] = src[i].clone();
            i += 1;
        }
    }
    pub fn copy_within<R: crate::ops::RangeBounds<usize>>(&mut self, src: R, dest: usize)
    where
        T: Copy,
    {
        let (start, end) = crate::ops::range_to_bounds(&src, self.len());
        let count = end - start;
        if dest > self.len() - count {
            panic!("dest is out of bounds");
        }
        unsafe {
            let p = self.as_mut_ptr();
            intrinsics::copy(p.add(start) as *const T, p.add(dest), count)
        }
    }
    pub fn swap_with_slice(&mut self, other: &mut [T]) {
        if self.len() != other.len() {
            panic!("destination and source slices have different lengths");
        }
        let mut i = 0;
        while i < self.len() {
            crate::mem::swap(&mut self[i], &mut other[i]);
            i += 1;
        }
    }
    pub fn rotate_left(&mut self, mid: usize) {
        let n = self.len();
        if mid > n {
            panic!("assertion failed: mid <= self.len()");
        }
        self[..mid].reverse();
        self[mid..].reverse();
        self.reverse();
    }
    pub fn rotate_right(&mut self, k: usize) {
        let n = self.len();
        if k > n {
            panic!("assertion failed: k <= self.len()");
        }
        self.rotate_left(n - k);
    }
    pub fn windows(&self, size: usize) -> Windows<'_, T> {
        if size == 0 {
            panic!("window size must be non-zero");
        }
        iter::windows_new(self, size)
    }
    pub fn chunks(&self, chunk_size: usize) -> Chunks<'_, T> {
        if chunk_size == 0 {
            panic!("chunk size must be non-zero");
        }
        iter::chunks_new(self, chunk_size)
    }
    pub fn chunks_mut(&mut self, chunk_size: usize) -> ChunksMut<'_, T> {
        if chunk_size == 0 {
            panic!("chunk size must be non-zero");
        }
        iter::chunks_mut_new(self, chunk_size)
    }
    pub fn chunks_exact(&self, chunk_size: usize) -> ChunksExact<'_, T> {
        if chunk_size == 0 {
            panic!("chunk size must be non-zero");
        }
        iter::chunks_exact_new(self, chunk_size)
    }
    pub fn rchunks(&self, chunk_size: usize) -> RChunks<'_, T> {
        if chunk_size == 0 {
            panic!("chunk size must be non-zero");
        }
        iter::rchunks_new(self, chunk_size)
    }
    pub fn chunk_by<F: FnMut(&T, &T) -> bool>(&self, pred: F) -> ChunkBy<'_, T, F> {
        iter::chunk_by_new(self, pred)
    }
    pub fn split<F: FnMut(&T) -> bool>(&self, pred: F) -> Split<'_, T, F> {
        iter::split_new(self, pred)
    }
    pub fn binary_search_by<F: FnMut(&T) -> Ordering>(&self, mut f: F) -> Result<usize, usize> {
        let mut lo = 0;
        let mut hi = self.len();
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
        self.binary_search_by(|p| p.cmp(x))
    }
    pub fn binary_search_by_key<B: Ord, F: FnMut(&T) -> B>(&self, b: &B, mut f: F) -> Result<usize, usize> {
        self.binary_search_by(|k| f(k).cmp(b))
    }
    pub fn partition_point<P: FnMut(&T) -> bool>(&self, mut pred: P) -> usize {
        let mut lo = 0;
        let mut hi = self.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if pred(&self[mid]) { lo = mid + 1 } else { hi = mid }
        }
        lo
    }
    pub fn sort(&mut self)
    where
        T: Ord,
    {
        sort::merge_sort(self, &mut |a: &T, b: &T| a.cmp(b));
    }
    pub fn sort_by<F: FnMut(&T, &T) -> Ordering>(&mut self, mut compare: F) {
        sort::merge_sort(self, &mut compare);
    }
    pub fn sort_by_key<K: Ord, F: FnMut(&T) -> K>(&mut self, mut f: F) {
        sort::merge_sort(self, &mut |a: &T, b: &T| f(a).cmp(&f(b)));
    }
    pub fn sort_by_cached_key<K: Ord, F: FnMut(&T) -> K>(&mut self, mut f: F) {
        sort::merge_sort(self, &mut |a: &T, b: &T| f(a).cmp(&f(b)));
    }
    pub fn sort_unstable(&mut self)
    where
        T: Ord,
    {
        sort::merge_sort(self, &mut |a: &T, b: &T| a.cmp(b));
    }
    pub fn sort_unstable_by<F: FnMut(&T, &T) -> Ordering>(&mut self, mut compare: F) {
        sort::merge_sort(self, &mut compare);
    }
    pub fn sort_unstable_by_key<K: Ord, F: FnMut(&T) -> K>(&mut self, mut f: F) {
        sort::merge_sort(self, &mut |a: &T, b: &T| f(a).cmp(&f(b)));
    }
    pub fn is_sorted(&self) -> bool
    where
        T: PartialOrd,
    {
        let mut i = 1;
        while i < self.len() {
            if self[i] < self[i - 1] {
                return false;
            }
            i += 1;
        }
        true
    }
    pub fn concat<Item: ?Sized>(&self) -> <[T] as Concat<Item>>::Output
    where
        [T]: Concat<Item>,
    {
        Concat::concat(self)
    }
    pub fn join<Separator>(&self, sep: Separator) -> <[T] as Join<Separator>>::Output
    where
        [T]: Join<Separator>,
    {
        Join::join(self, sep)
    }
    pub fn to_ascii_lowercase_bytes(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.to_vec()
    }
}

impl [u8] {
    pub fn is_ascii(&self) -> bool {
        let mut i = 0;
        while i < self.len() {
            if self[i] >= 128 {
                return false;
            }
            i += 1;
        }
        true
    }
    pub fn eq_ignore_ascii_case(&self, other: &[u8]) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len() {
            if self[i].to_ascii_lowercase() != other[i].to_ascii_lowercase() {
                return false;
            }
            i += 1;
        }
        true
    }
    pub fn to_ascii_lowercase(&self) -> Vec<u8> {
        let mut v = self.to_vec();
        v.make_ascii_lowercase();
        v
    }
    pub fn to_ascii_uppercase(&self) -> Vec<u8> {
        let mut v = self.to_vec();
        v.make_ascii_uppercase();
        v
    }
    pub fn make_ascii_lowercase(&mut self) {
        let mut i = 0;
        while i < self.len() {
            self[i] = self[i].to_ascii_lowercase();
            i += 1;
        }
    }
    pub fn make_ascii_uppercase(&mut self) {
        let mut i = 0;
        while i < self.len() {
            self[i] = self[i].to_ascii_uppercase();
            i += 1;
        }
    }
    pub fn trim_ascii(&self) -> &[u8] {
        let mut a = 0;
        let mut b = self.len();
        while a < b && self[a].is_ascii_whitespace() {
            a += 1;
        }
        while b > a && self[b - 1].is_ascii_whitespace() {
            b -= 1;
        }
        &self[a..b]
    }
}

impl<T, I: SliceIndex<[T]>> crate::ops::Index<I> for [T] {
    type Output = I::Output;
    fn index(&self, index: I) -> &I::Output {
        index.index(self)
    }
}
impl<T, I: SliceIndex<[T]>> crate::ops::IndexMut<I> for [T] {
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        index.index_mut(self)
    }
}

impl<T: PartialEq<U>, U> PartialEq<[U]> for [T] {
    fn eq(&self, other: &[U]) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len() {
            if self[i] != other[i] {
                return false;
            }
            i += 1;
        }
        true
    }
}
impl<T: Eq> Eq for [T] {}

impl<T: PartialOrd> PartialOrd for [T] {
    fn partial_cmp(&self, other: &[T]) -> Option<Ordering> {
        let n = if self.len() < other.len() { self.len() } else { other.len() };
        let mut i = 0;
        while i < n {
            match self[i].partial_cmp(&other[i]) {
                Some(Ordering::Equal) => {}
                r => return r,
            }
            i += 1;
        }
        self.len().partial_cmp(&other.len())
    }
}
impl<T: Ord> Ord for [T] {
    fn cmp(&self, other: &[T]) -> Ordering {
        let n = if self.len() < other.len() { self.len() } else { other.len() };
        let mut i = 0;
        while i < n {
            match self[i].cmp(&other[i]) {
                Ordering::Equal => {}
                r => return r,
            }
            i += 1;
        }
        self.len().cmp(&other.len())
    }
}

impl<'a, T> IntoIterator for &'a [T] {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}
impl<'a, T> IntoIterator for &'a mut [T] {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<T> Default for &[T] {
    fn default() -> &'static [T] {
        &[]
    }
}

impl<T: Clone> crate::borrow::ToOwned for [T] {
    type Owned = Vec<T>;
    fn to_owned(&self) -> Vec<T> {
        self.to_vec()
    }
}

impl<T> AsRef<[T]> for [T] {
    fn as_ref(&self) -> &[T] {
        self
    }
}
impl<T> AsMut<[T]> for [T] {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}
