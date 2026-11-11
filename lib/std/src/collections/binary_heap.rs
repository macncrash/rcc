//! `BinaryHeap<T>`: a max-heap in a `Vec`. The sift algorithms (and the
//! comparisons they make) follow real std exactly, so the internal order
//! (visible through `{:?}`, `iter`, `into_vec`) matches rustc's.

use crate::fmt;
use crate::iter::{FromIterator, FusedIterator};
use crate::mem;
use crate::ops::{Deref, DerefMut};
use crate::slice;

pub struct BinaryHeap<T> {
    data: Vec<T>,
}

impl<T> BinaryHeap<T> {
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { iter: self.data.iter() }
    }
    pub fn peek(&self) -> Option<&T> {
        self.data.get(0)
    }
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional)
    }
    pub fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional)
    }
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit()
    }
    pub fn into_vec(self) -> Vec<T> {
        self.data
    }
    pub fn as_slice(&self) -> &[T] {
        self.data.as_slice()
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain { iter: self.data.drain(..), marker: crate::marker::PhantomData }
    }
    pub fn clear(&mut self) {
        self.data.clear()
    }
}

impl<T: Ord> BinaryHeap<T> {
    pub fn new() -> BinaryHeap<T> {
        BinaryHeap { data: Vec::new() }
    }
    pub fn with_capacity(capacity: usize) -> BinaryHeap<T> {
        BinaryHeap { data: Vec::with_capacity(capacity) }
    }
    pub fn peek_mut(&mut self) -> Option<PeekMut<'_, T>> {
        if self.is_empty() { None } else { Some(PeekMut { heap: self, modified: false }) }
    }
    pub fn pop(&mut self) -> Option<T> {
        match self.data.pop() {
            None => None,
            Some(mut item) => {
                if !self.is_empty() {
                    mem::swap(&mut item, &mut self.data[0]);
                    self.sift_down_to_bottom(0);
                }
                Some(item)
            }
        }
    }
    pub fn push(&mut self, item: T) {
        let old_len = self.len();
        self.data.push(item);
        self.sift_up(0, old_len);
    }
    pub fn into_sorted_vec(mut self) -> Vec<T> {
        let mut end = self.len();
        while end > 1 {
            end -= 1;
            self.data.swap(0, end);
            self.sift_down_range(0, end);
        }
        self.into_vec()
    }
    pub fn append(&mut self, other: &mut BinaryHeap<T>) {
        if self.len() < other.len() {
            mem::swap(self, other);
        }
        let start = self.data.len();
        self.data.append(&mut other.data);
        self.rebuild_tail(start);
    }
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut f: F) {
        let mut rebuild_from = self.len();
        let mut i = 0;
        self.data.retain(|e| {
            let keep = f(e);
            if !keep && i < rebuild_from {
                rebuild_from = i;
            }
            i += 1;
            keep
        });
        self.rebuild_tail(rebuild_from);
    }
    pub fn into_iter_sorted(self) -> IntoIterSorted<T> {
        IntoIterSorted { inner: self }
    }
    pub fn drain_sorted(&mut self) -> IntoIterSorted<T> {
        IntoIterSorted { inner: mem::replace(self, BinaryHeap::new()) }
    }

    // `hole` in std moves the element instead of swapping; the comparisons
    // and the final arrangement are the same.
    fn sift_up(&mut self, start: usize, pos: usize) -> usize {
        let mut hole = pos;
        while hole > start {
            let parent = (hole - 1) / 2;
            if self.data[hole] <= self.data[parent] {
                break;
            }
            self.data.swap(hole, parent);
            hole = parent;
        }
        hole
    }
    fn sift_down_range(&mut self, pos: usize, end: usize) {
        let mut hole = pos;
        let mut child = 2 * hole + 1;
        while child <= end.saturating_sub(2) {
            if self.data[child] <= self.data[child + 1] {
                child += 1;
            }
            if self.data[hole] >= self.data[child] {
                return;
            }
            self.data.swap(hole, child);
            hole = child;
            child = 2 * hole + 1;
        }
        if child == end.wrapping_sub(1) && self.data[hole] < self.data[child] {
            self.data.swap(hole, child);
        }
    }
    fn sift_down(&mut self, pos: usize) {
        let len = self.len();
        self.sift_down_range(pos, len);
    }
    fn sift_down_to_bottom(&mut self, pos: usize) {
        let end = self.len();
        let start = pos;
        let mut hole = pos;
        let mut child = 2 * hole + 1;
        while child <= end.saturating_sub(2) {
            if self.data[child] <= self.data[child + 1] {
                child += 1;
            }
            self.data.swap(hole, child);
            hole = child;
            child = 2 * hole + 1;
        }
        if child == end - 1 {
            self.data.swap(hole, child);
            hole = child;
        }
        self.sift_up(start, hole);
    }
    fn rebuild_tail(&mut self, start: usize) {
        if start == self.len() {
            return;
        }
        let tail_len = self.len() - start;
        let better_to_rebuild = if start < tail_len {
            true
        } else if self.len() <= 2048 {
            let log2 = (usize::BITS - start.leading_zeros() - 1) as usize;
            2 * self.len() < tail_len * log2
        } else {
            2 * self.len() < tail_len * 11
        };
        if better_to_rebuild {
            self.rebuild();
        } else {
            let mut i = start;
            while i < self.len() {
                self.sift_up(0, i);
                i += 1;
            }
        }
    }
    fn rebuild(&mut self) {
        let mut n = self.len() / 2;
        while n > 0 {
            n -= 1;
            self.sift_down(n);
        }
    }
}

/// Mutable access to the greatest element; restores the heap on drop.
pub struct PeekMut<'a, T: Ord> {
    heap: &'a mut BinaryHeap<T>,
    modified: bool,
}
impl<'a, T: Ord> PeekMut<'a, T> {
    pub fn pop(mut this: PeekMut<'a, T>) -> T {
        this.modified = false;
        match this.heap.pop() {
            Some(x) => x,
            None => panic!("PeekMut::pop on empty heap"),
        }
    }
}
impl<'a, T: Ord> Drop for PeekMut<'a, T> {
    fn drop(&mut self) {
        if self.modified {
            self.heap.sift_down(0);
        }
    }
}
impl<'a, T: Ord> Deref for PeekMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.heap.data[0]
    }
}
impl<'a, T: Ord> DerefMut for PeekMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.modified = true;
        &mut self.heap.data[0]
    }
}
impl<'a, T: Ord + fmt::Debug> fmt::Debug for PeekMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PeekMut").field(&self.heap.data[0]).finish()
    }
}

impl<T: Clone> Clone for BinaryHeap<T> {
    fn clone(&self) -> BinaryHeap<T> {
        BinaryHeap { data: self.data.clone() }
    }
}
impl<T: Ord> Default for BinaryHeap<T> {
    fn default() -> BinaryHeap<T> {
        BinaryHeap::new()
    }
}
impl<T: fmt::Debug> fmt::Debug for BinaryHeap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
impl<T: Ord> From<Vec<T>> for BinaryHeap<T> {
    fn from(vec: Vec<T>) -> BinaryHeap<T> {
        let mut heap = BinaryHeap { data: vec };
        heap.rebuild();
        heap
    }
}
impl<T: Ord, const N: usize> From<[T; N]> for BinaryHeap<T> {
    fn from(arr: [T; N]) -> BinaryHeap<T> {
        BinaryHeap::from(crate::vec::from_array(arr))
    }
}
impl<T> From<BinaryHeap<T>> for Vec<T> {
    fn from(heap: BinaryHeap<T>) -> Vec<T> {
        heap.data
    }
}
impl<T: Ord> FromIterator<T> for BinaryHeap<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> BinaryHeap<T> {
        let v: Vec<T> = Vec::from_iter(iter);
        BinaryHeap::from(v)
    }
}
impl<T: Ord> Extend<T> for BinaryHeap<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let start = self.data.len();
        self.data.extend(iter);
        self.rebuild_tail(start);
    }
}
impl<'a, T: 'a + Ord + Copy> Extend<&'a T> for BinaryHeap<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        let start = self.data.len();
        for x in iter {
            self.data.push(*x);
        }
        self.rebuild_tail(start);
    }
}

impl<'a, T> IntoIterator for &'a BinaryHeap<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}
impl<T> IntoIterator for BinaryHeap<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { iter: self.data.into_iter() }
    }
}

pub struct Iter<'a, T> {
    iter: slice::Iter<'a, T>,
}
impl<'a, T> Clone for Iter<'a, T> {
    fn clone(&self) -> Iter<'a, T> {
        Iter { iter: self.iter.clone() }
    }
}
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        self.iter.next_back()
    }
}
impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
impl<'a, T> FusedIterator for Iter<'a, T> {}

pub struct IntoIter<T> {
    iter: crate::vec::IntoIter<T>,
}
impl<T> Iterator for IntoIter<T> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}
impl<T> ExactSizeIterator for IntoIter<T> {}
impl<T> FusedIterator for IntoIter<T> {}

pub struct IntoIterSorted<T> {
    inner: BinaryHeap<T>,
}
impl<T: Ord> Iterator for IntoIterSorted<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.inner.pop()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.inner.len();
        (n, Some(n))
    }
}

pub struct Drain<'a, T> {
    iter: crate::vec::Drain<T>,
    marker: crate::marker::PhantomData<&'a mut T>,
}
impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<'a, T> DoubleEndedIterator for Drain<'a, T> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}
impl<'a, T> ExactSizeIterator for Drain<'a, T> {}
impl<'a, T> FusedIterator for Drain<'a, T> {}
