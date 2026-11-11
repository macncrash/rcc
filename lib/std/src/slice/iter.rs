use crate::iter::{DoubleEndedIterator, ExactSizeIterator, Iterator};
use crate::marker::PhantomData;
use crate::slice::{from_raw_parts, from_raw_parts_mut};

/// Immutable slice iterator: a [ptr, end) pointer pair.
pub struct Iter<'a, T> {
    ptr: *const T,
    end: *const T,
    marker: PhantomData<&'a T>,
}

pub(crate) fn iter_new<'a, T>(s: &'a [T]) -> Iter<'a, T> {
    let p = s.as_ptr();
    Iter { ptr: p, end: unsafe { p.add(s.len()) }, marker: PhantomData }
}

impl<'a, T> Iter<'a, T> {
    pub fn as_slice(&self) -> &'a [T] {
        unsafe { from_raw_parts(self.ptr, self.remaining()) }
    }
    fn remaining(&self) -> usize {
        unsafe { crate::intrinsics::ptr_diff(self.end as *const T, self.ptr as *const T) as usize }
    }
}

impl<'a, T> Clone for Iter<'a, T> {
    fn clone(&self) -> Iter<'a, T> {
        Iter { ptr: self.ptr, end: self.end, marker: PhantomData }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        if (self.ptr as usize) == (self.end as usize) {
            None
        } else {
            unsafe {
                let p = self.ptr;
                self.ptr = p.add(1);
                Some(&*p)
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.remaining();
        (n, Some(n))
    }
    fn count(self) -> usize {
        self.remaining()
    }
    fn nth(&mut self, n: usize) -> Option<&'a T> {
        if n >= self.remaining() {
            self.ptr = self.end;
            return None;
        }
        unsafe {
            let p = self.ptr.add(n);
            self.ptr = p.add(1);
            Some(&*p)
        }
    }
}
impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        if (self.ptr as usize) == (self.end as usize) {
            None
        } else {
            unsafe {
                self.end = self.end.sub(1);
                Some(&*self.end)
            }
        }
    }
}
impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.remaining()
    }
}

pub struct IterMut<'a, T> {
    ptr: *mut T,
    end: *mut T,
    marker: PhantomData<&'a mut T>,
}

pub(crate) fn iter_mut_new<'a, T>(s: &'a mut [T]) -> IterMut<'a, T> {
    let n = s.len();
    let p = s.as_mut_ptr();
    IterMut { ptr: p, end: unsafe { p.add(n) }, marker: PhantomData }
}

impl<'a, T> IterMut<'a, T> {
    fn remaining(&self) -> usize {
        unsafe { crate::intrinsics::ptr_diff(self.end as *const T, self.ptr as *const T) as usize }
    }
    pub fn into_slice(self) -> &'a mut [T] {
        unsafe { from_raw_parts_mut(self.ptr, self.remaining()) }
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T> {
        if (self.ptr as usize) == (self.end as usize) {
            None
        } else {
            unsafe {
                let p = self.ptr;
                self.ptr = p.add(1);
                Some(&mut *p)
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.remaining();
        (n, Some(n))
    }
}
impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<&'a mut T> {
        if (self.ptr as usize) == (self.end as usize) {
            None
        } else {
            unsafe {
                self.end = self.end.sub(1);
                Some(&mut *self.end)
            }
        }
    }
}
impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.remaining()
    }
}

pub struct Windows<'a, T> {
    v: &'a [T],
    size: usize,
}
pub(crate) fn windows_new<'a, T>(v: &'a [T], size: usize) -> Windows<'a, T> {
    Windows { v, size }
}
impl<'a, T> Iterator for Windows<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.size > self.v.len() {
            None
        } else {
            let r = &self.v[..self.size];
            self.v = &self.v[1..];
            Some(r)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = if self.size > self.v.len() { 0 } else { self.v.len() - self.size + 1 };
        (n, Some(n))
    }
}
impl<'a, T> ExactSizeIterator for Windows<'a, T> {}
impl<'a, T> DoubleEndedIterator for Windows<'a, T> {
    fn next_back(&mut self) -> Option<&'a [T]> {
        if self.size > self.v.len() {
            None
        } else {
            let n = self.v.len();
            let r = &self.v[n - self.size..];
            self.v = &self.v[..n - 1];
            Some(r)
        }
    }
}

pub struct Chunks<'a, T> {
    v: &'a [T],
    size: usize,
}
pub(crate) fn chunks_new<'a, T>(v: &'a [T], size: usize) -> Chunks<'a, T> {
    Chunks { v, size }
}
impl<'a, T> Iterator for Chunks<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.v.len() == 0 {
            return None;
        }
        let n = if self.size < self.v.len() { self.size } else { self.v.len() };
        let r = &self.v[..n];
        self.v = &self.v[n..];
        Some(r)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.v.len() + self.size - 1) / self.size;
        (n, Some(n))
    }
}
impl<'a, T> ExactSizeIterator for Chunks<'a, T> {}
impl<'a, T> DoubleEndedIterator for Chunks<'a, T> {
    fn next_back(&mut self) -> Option<&'a [T]> {
        let len = self.v.len();
        if len == 0 {
            return None;
        }
        let rem = len % self.size;
        let n = if rem == 0 { self.size } else { rem };
        let r = &self.v[len - n..];
        self.v = &self.v[..len - n];
        Some(r)
    }
}

pub struct ChunksMut<'a, T> {
    ptr: *mut T,
    len: usize,
    size: usize,
    marker: PhantomData<&'a mut T>,
}
pub(crate) fn chunks_mut_new<'a, T>(v: &'a mut [T], size: usize) -> ChunksMut<'a, T> {
    let len = v.len();
    ChunksMut { ptr: v.as_mut_ptr(), len, size, marker: PhantomData }
}
impl<'a, T> Iterator for ChunksMut<'a, T> {
    type Item = &'a mut [T];
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.len == 0 {
            return None;
        }
        let n = if self.size < self.len { self.size } else { self.len };
        unsafe {
            let r = from_raw_parts_mut(self.ptr, n);
            self.ptr = self.ptr.add(n);
            self.len -= n;
            Some(r)
        }
    }
}

pub struct ChunksExact<'a, T> {
    v: &'a [T],
    rem: &'a [T],
    size: usize,
}
pub(crate) fn chunks_exact_new<'a, T>(v: &'a [T], size: usize) -> ChunksExact<'a, T> {
    let full = v.len() - v.len() % size;
    ChunksExact { v: &v[..full], rem: &v[full..], size }
}
impl<'a, T> ChunksExact<'a, T> {
    pub fn remainder(&self) -> &'a [T] {
        self.rem
    }
}
impl<'a, T> Iterator for ChunksExact<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.v.len() < self.size {
            return None;
        }
        let r = &self.v[..self.size];
        self.v = &self.v[self.size..];
        Some(r)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.v.len() / self.size;
        (n, Some(n))
    }
}
impl<'a, T> ExactSizeIterator for ChunksExact<'a, T> {}

pub struct RChunks<'a, T> {
    v: &'a [T],
    size: usize,
}
pub(crate) fn rchunks_new<'a, T>(v: &'a [T], size: usize) -> RChunks<'a, T> {
    RChunks { v, size }
}
impl<'a, T> Iterator for RChunks<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        let len = self.v.len();
        if len == 0 {
            return None;
        }
        let n = if self.size < len { self.size } else { len };
        let r = &self.v[len - n..];
        self.v = &self.v[..len - n];
        Some(r)
    }
}

pub struct ChunkBy<'a, T, F> {
    v: &'a [T],
    pred: F,
}
pub(crate) fn chunk_by_new<'a, T, F>(v: &'a [T], pred: F) -> ChunkBy<'a, T, F> {
    ChunkBy { v, pred }
}
impl<'a, T, F: FnMut(&T, &T) -> bool> Iterator for ChunkBy<'a, T, F> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.v.len() == 0 {
            return None;
        }
        let mut n = 1;
        while n < self.v.len() && (self.pred)(&self.v[n - 1], &self.v[n]) {
            n += 1;
        }
        let r = &self.v[..n];
        self.v = &self.v[n..];
        Some(r)
    }
}

pub struct Split<'a, T, F> {
    v: &'a [T],
    pred: F,
    finished: bool,
}
pub(crate) fn split_new<'a, T, F>(v: &'a [T], pred: F) -> Split<'a, T, F> {
    Split { v, pred, finished: false }
}
impl<'a, T, F: FnMut(&T) -> bool> Iterator for Split<'a, T, F> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.finished {
            return None;
        }
        let mut i = 0;
        while i < self.v.len() {
            if (self.pred)(&self.v[i]) {
                let r = &self.v[..i];
                self.v = &self.v[i + 1..];
                return Some(r);
            }
            i += 1;
        }
        self.finished = true;
        Some(self.v)
    }
}
