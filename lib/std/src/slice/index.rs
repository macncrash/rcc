use crate::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use crate::slice::{from_raw_parts, from_raw_parts_mut};

/// A value usable to index a slice (or str): `usize` and the range types.
pub trait SliceIndex<T: ?Sized> {
    type Output: ?Sized;
    fn get(self, slice: &T) -> Option<&Self::Output>;
    fn get_mut(self, slice: &mut T) -> Option<&mut Self::Output>;
    fn index(self, slice: &T) -> &Self::Output;
    fn index_mut(self, slice: &mut T) -> &mut Self::Output;
}

fn start_fail(start: usize, len: usize) -> ! {
    panic!("range start index {} out of range for slice of length {}", start, len)
}
fn end_fail(end: usize, len: usize) -> ! {
    panic!("range end index {} out of range for slice of length {}", end, len)
}
fn order_fail(start: usize, end: usize) -> ! {
    panic!("slice index starts at {} but ends at {}", start, end)
}

impl<T> SliceIndex<[T]> for usize {
    type Output = T;
    fn get(self, slice: &[T]) -> Option<&T> {
        if self < slice.len() { unsafe { Some(&*slice.as_ptr().add(self)) } } else { None }
    }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut T> {
        if self < slice.len() { unsafe { Some(&mut *slice.as_mut_ptr().add(self)) } } else { None }
    }
    fn index(self, slice: &[T]) -> &T {
        &slice[self]
    }
    fn index_mut(self, slice: &mut [T]) -> &mut T {
        &mut slice[self]
    }
}

impl<T> SliceIndex<[T]> for Range<usize> {
    type Output = [T];
    fn get(self, slice: &[T]) -> Option<&[T]> {
        if self.start > self.end || self.end > slice.len() {
            None
        } else {
            unsafe { Some(from_raw_parts(slice.as_ptr().add(self.start), self.end - self.start)) }
        }
    }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        if self.start > self.end || self.end > slice.len() {
            None
        } else {
            unsafe { Some(from_raw_parts_mut(slice.as_mut_ptr().add(self.start), self.end - self.start)) }
        }
    }
    fn index(self, slice: &[T]) -> &[T] {
        if self.start > self.end {
            order_fail(self.start, self.end);
        }
        if self.end > slice.len() {
            end_fail(self.end, slice.len());
        }
        unsafe { from_raw_parts(slice.as_ptr().add(self.start), self.end - self.start) }
    }
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        if self.start > self.end {
            order_fail(self.start, self.end);
        }
        if self.end > slice.len() {
            end_fail(self.end, slice.len());
        }
        unsafe { from_raw_parts_mut(slice.as_mut_ptr().add(self.start), self.end - self.start) }
    }
}

impl<T> SliceIndex<[T]> for RangeTo<usize> {
    type Output = [T];
    fn get(self, slice: &[T]) -> Option<&[T]> {
        (0..self.end).get(slice)
    }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        (0..self.end).get_mut(slice)
    }
    fn index(self, slice: &[T]) -> &[T] {
        (0..self.end).index(slice)
    }
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        (0..self.end).index_mut(slice)
    }
}

impl<T> SliceIndex<[T]> for RangeFrom<usize> {
    type Output = [T];
    fn get(self, slice: &[T]) -> Option<&[T]> {
        (self.start..slice.len()).get(slice)
    }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        let n = slice.len();
        (self.start..n).get_mut(slice)
    }
    fn index(self, slice: &[T]) -> &[T] {
        if self.start > slice.len() {
            start_fail(self.start, slice.len());
        }
        (self.start..slice.len()).index(slice)
    }
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        let n = slice.len();
        if self.start > n {
            start_fail(self.start, n);
        }
        (self.start..n).index_mut(slice)
    }
}

impl<T> SliceIndex<[T]> for RangeFull {
    type Output = [T];
    fn get(self, slice: &[T]) -> Option<&[T]> {
        Some(slice)
    }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        Some(slice)
    }
    fn index(self, slice: &[T]) -> &[T] {
        slice
    }
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        slice
    }
}

impl<T> SliceIndex<[T]> for RangeInclusive<usize> {
    type Output = [T];
    fn get(self, slice: &[T]) -> Option<&[T]> {
        if self.end == usize::MAX { None } else { (self.start..self.end + 1).get(slice) }
    }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        if self.end == usize::MAX { None } else { (self.start..self.end + 1).get_mut(slice) }
    }
    fn index(self, slice: &[T]) -> &[T] {
        if self.end == usize::MAX {
            end_fail(self.end, slice.len());
        }
        (self.start..self.end + 1).index(slice)
    }
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        if self.end == usize::MAX {
            end_fail(self.end, slice.len());
        }
        (self.start..self.end + 1).index_mut(slice)
    }
}

impl<T> SliceIndex<[T]> for RangeToInclusive<usize> {
    type Output = [T];
    fn get(self, slice: &[T]) -> Option<&[T]> {
        (0..=self.end).get(slice)
    }
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        (0..=self.end).get_mut(slice)
    }
    fn index(self, slice: &[T]) -> &[T] {
        (0..=self.end).index(slice)
    }
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        (0..=self.end).index_mut(slice)
    }
}
