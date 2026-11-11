//! Fixed-size arrays `[T; N]`.

use crate::cmp::Ordering;
use crate::intrinsics;
use crate::slice;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TryFromSliceError(());

impl crate::fmt::Display for TryFromSliceError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("could not convert slice to array")
    }
}

/// Build an array by calling `f(i)` for each index.
pub fn from_fn<T, const N: usize, F: FnMut(usize) -> T>(mut f: F) -> [T; N] {
    unsafe {
        let mut a = crate::mem::MaybeUninit::<[T; N]>::uninit();
        let p = a.as_mut_ptr() as *mut T;
        let mut i = 0;
        while i < N {
            intrinsics::write(p.add(i), f(i));
            i += 1;
        }
        a.assume_init()
    }
}

impl<T, const N: usize> [T; N] {
    pub const fn as_slice(&self) -> &[T] {
        self
    }
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> [U; N] {
        unsafe {
            let mut out = crate::mem::MaybeUninit::<[U; N]>::uninit();
            let src = &self as *const [T; N] as *const T;
            let dst = out.as_mut_ptr() as *mut U;
            let mut i = 0;
            while i < N {
                intrinsics::write(dst.add(i), f(intrinsics::read(src.add(i))));
                i += 1;
            }
            intrinsics::forget(self);
            out.assume_init()
        }
    }
    pub fn each_ref(&self) -> [&T; N] {
        from_fn(|i| &self[i])
    }
}

impl<T: Clone, const N: usize> Clone for [T; N] {
    fn clone(&self) -> [T; N] {
        from_fn(|i| self[i].clone())
    }
}

impl<T: Default, const N: usize> Default for [T; N] {
    fn default() -> [T; N] {
        from_fn(|_i| T::default())
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for [T; N] {
    fn eq(&self, other: &[U; N]) -> bool {
        let a: &[T] = self;
        let b: &[U] = other;
        a == b
    }
}
impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U]> for [T; N] {
    fn eq(&self, other: &[U]) -> bool {
        let a: &[T] = self;
        a == other
    }
}
impl<T: PartialEq<U>, U, const N: usize> PartialEq<&[U]> for [T; N] {
    fn eq(&self, other: &&[U]) -> bool {
        let a: &[T] = self;
        a == *other
    }
}
impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for [T] {
    fn eq(&self, other: &[U; N]) -> bool {
        let b: &[U] = other;
        self == b
    }
}
impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for &[T] {
    fn eq(&self, other: &[U; N]) -> bool {
        let b: &[U] = other;
        *self == b
    }
}
impl<T: Eq, const N: usize> Eq for [T; N] {}
impl<T: PartialOrd, const N: usize> PartialOrd for [T; N] {
    fn partial_cmp(&self, other: &[T; N]) -> Option<Ordering> {
        let a: &[T] = self;
        let b: &[T] = other;
        a.partial_cmp(b)
    }
}
impl<T: Ord, const N: usize> Ord for [T; N] {
    fn cmp(&self, other: &[T; N]) -> Ordering {
        let a: &[T] = self;
        let b: &[T] = other;
        a.cmp(b)
    }
}

impl<T, I, const N: usize> crate::ops::Index<I> for [T; N]
where
    [T]: crate::ops::Index<I>,
{
    type Output = <[T] as crate::ops::Index<I>>::Output;
    fn index(&self, index: I) -> &<[T] as crate::ops::Index<I>>::Output {
        let s: &[T] = self;
        crate::ops::Index::index(s, index)
    }
}
impl<T, I, const N: usize> crate::ops::IndexMut<I> for [T; N]
where
    [T]: crate::ops::IndexMut<I>,
{
    fn index_mut(&mut self, index: I) -> &mut <[T] as crate::ops::Index<I>>::Output {
        let s: &mut [T] = self;
        crate::ops::IndexMut::index_mut(s, index)
    }
}

impl<T, const N: usize> AsRef<[T]> for [T; N] {
    fn as_ref(&self) -> &[T] {
        self
    }
}
impl<T, const N: usize> AsMut<[T]> for [T; N] {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}
impl<T, const N: usize> crate::borrow::Borrow<[T]> for [T; N] {
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T: Copy, const N: usize> TryFrom<&[T]> for [T; N] {
    type Error = TryFromSliceError;
    fn try_from(s: &[T]) -> Result<[T; N], TryFromSliceError> {
        if s.len() == N { Ok(unsafe { intrinsics::read(s.as_ptr() as *const [T; N]) }) } else { Err(TryFromSliceError(())) }
    }
}
impl<T: Copy, const N: usize> TryFrom<&mut [T]> for [T; N] {
    type Error = TryFromSliceError;
    fn try_from(s: &mut [T]) -> Result<[T; N], TryFromSliceError> {
        if s.len() == N { Ok(unsafe { intrinsics::read(s.as_ptr() as *const [T; N]) }) } else { Err(TryFromSliceError(())) }
    }
}
impl<'a, T, const N: usize> TryFrom<&'a [T]> for &'a [T; N] {
    type Error = TryFromSliceError;
    fn try_from(s: &'a [T]) -> Result<&'a [T; N], TryFromSliceError> {
        if s.len() == N { Ok(unsafe { &*(s.as_ptr() as *const [T; N]) }) } else { Err(TryFromSliceError(())) }
    }
}
impl<T: Copy, const N: usize> TryFrom<crate::vec::Vec<T>> for [T; N] {
    type Error = crate::vec::Vec<T>;
    fn try_from(v: crate::vec::Vec<T>) -> Result<[T; N], crate::vec::Vec<T>> {
        if v.len() == N { Ok(unsafe { intrinsics::read(v.as_ptr() as *const [T; N]) }) } else { Err(v) }
    }
}

/// By-value array iterator.
pub struct IntoIter<T, const N: usize> {
    data: crate::mem::MaybeUninit<[T; N]>,
    pos: usize,
    end: usize,
}

impl<T, const N: usize> Iterator for IntoIter<T, N> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.pos == self.end {
            None
        } else {
            let i = self.pos;
            self.pos += 1;
            unsafe { Some(intrinsics::read((self.data.as_ptr() as *const T).add(i))) }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.end - self.pos, Some(self.end - self.pos))
    }
}
impl<T, const N: usize> DoubleEndedIterator for IntoIter<T, N> {
    fn next_back(&mut self) -> Option<T> {
        if self.pos == self.end {
            None
        } else {
            self.end -= 1;
            unsafe { Some(intrinsics::read((self.data.as_ptr() as *const T).add(self.end))) }
        }
    }
}
impl<T, const N: usize> ExactSizeIterator for IntoIter<T, N> {}
impl<T, const N: usize> Drop for IntoIter<T, N> {
    fn drop(&mut self) {
        unsafe {
            let p = (self.data.as_mut_ptr() as *mut T).add(self.pos);
            intrinsics::drop_in_place(crate::ptr::slice_from_raw_parts_mut(p, self.end - self.pos));
        }
    }
}

impl<T, const N: usize> IntoIterator for [T; N] {
    type Item = T;
    type IntoIter = IntoIter<T, N>;
    fn into_iter(self) -> IntoIter<T, N> {
        IntoIter { data: crate::mem::MaybeUninit::new(self), pos: 0, end: N }
    }
}
impl<'a, T, const N: usize> IntoIterator for &'a [T; N] {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;
    fn into_iter(self) -> slice::Iter<'a, T> {
        let s: &[T] = self;
        s.iter()
    }
}
impl<'a, T, const N: usize> IntoIterator for &'a mut [T; N] {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;
    fn into_iter(self) -> slice::IterMut<'a, T> {
        let s: &mut [T] = self;
        s.iter_mut()
    }
}
