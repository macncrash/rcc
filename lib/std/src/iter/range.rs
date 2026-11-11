//! Iteration over ranges. As in real std there is ONE generic impl per range
//! type (over `Step`), so that `for i in 0..n` gives `i` the type of the
//! range's bounds even while that is still an integer inference variable.

use crate::iter::traits::{DoubleEndedIterator, ExactSizeIterator, Iterator};
use crate::ops::{Range, RangeFrom, RangeInclusive};

pub trait Step: Clone + PartialOrd {
    /// Number of steps from start to end (0 if start >= end).
    fn steps_between(start: &Self, end: &Self) -> usize;
    fn forward(start: Self, n: usize) -> Self;
    fn backward(start: Self, n: usize) -> Self;
}

macro_rules! step_int {
    ($($t:ty)*) => { $(
        impl Step for $t {
            fn steps_between(start: &$t, end: &$t) -> usize {
                if *start < *end { (*end - *start) as usize } else { 0 }
            }
            fn forward(start: $t, n: usize) -> $t { start + (n as $t) }
            fn backward(start: $t, n: usize) -> $t { start - (n as $t) }
        }
    )* };
}
step_int! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize }

impl Step for char {
    fn steps_between(start: &char, end: &char) -> usize {
        let (a, b) = (*start as u32, *end as u32);
        if a >= b {
            return 0;
        }
        let mut n = b - a;
        if a < 0xD800 && b >= 0xE000 {
            n -= 0x800;
        }
        n as usize
    }
    fn forward(start: char, n: usize) -> char {
        let a = start as u32;
        let mut r = a + n as u32;
        if a < 0xD800 && r >= 0xD800 {
            r += 0x800;
        }
        unsafe { crate::char::from_u32_unchecked(r) }
    }
    fn backward(start: char, n: usize) -> char {
        let a = start as u32;
        let mut r = a - n as u32;
        if a >= 0xE000 && r < 0xE000 {
            r -= 0x800;
        }
        unsafe { crate::char::from_u32_unchecked(r) }
    }
}

impl<A: Step> Iterator for Range<A> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        if self.start < self.end {
            let v = self.start.clone();
            self.start = Step::forward(v.clone(), 1);
            Some(v)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = Step::steps_between(&self.start, &self.end);
        (n, Some(n))
    }
    fn nth(&mut self, n: usize) -> Option<A> {
        if n >= Step::steps_between(&self.start, &self.end) {
            self.start = self.end.clone();
            return None;
        }
        let v = Step::forward(self.start.clone(), n);
        self.start = Step::forward(v.clone(), 1);
        Some(v)
    }
}
impl<A: Step> DoubleEndedIterator for Range<A> {
    fn next_back(&mut self) -> Option<A> {
        if self.start < self.end {
            self.end = Step::backward(self.end.clone(), 1);
            Some(self.end.clone())
        } else {
            None
        }
    }
}
impl<A: Step> ExactSizeIterator for Range<A> {}

impl<A: Step> Iterator for RangeFrom<A> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        let v = self.start.clone();
        self.start = Step::forward(v.clone(), 1);
        Some(v)
    }
}

impl<A: Step> Iterator for RangeInclusive<A> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        if self.exhausted || self.start > self.end {
            return None;
        }
        let v = self.start.clone();
        if v == self.end {
            self.exhausted = true;
        } else {
            self.start = Step::forward(v.clone(), 1);
        }
        Some(v)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.exhausted || self.start > self.end {
            return (0, Some(0));
        }
        let n = Step::steps_between(&self.start, &self.end) + 1;
        (n, Some(n))
    }
}
impl<A: Step> DoubleEndedIterator for RangeInclusive<A> {
    fn next_back(&mut self) -> Option<A> {
        if self.exhausted || self.start > self.end {
            return None;
        }
        let v = self.end.clone();
        if v == self.start {
            self.exhausted = true;
        } else {
            self.end = Step::backward(v.clone(), 1);
        }
        Some(v)
    }
}
impl<A: Step> ExactSizeIterator for RangeInclusive<A> {}
