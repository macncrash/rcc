use crate::cmp::PartialOrd;

#[lang = "RangeFull"]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RangeFull;

#[lang = "Range"]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Range<Idx> {
    pub start: Idx,
    pub end: Idx,
}

#[lang = "RangeFrom"]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RangeFrom<Idx> {
    pub start: Idx,
}

#[lang = "RangeTo"]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RangeTo<Idx> {
    pub end: Idx,
}

/// `start..=end`. `exhausted` is set once iteration has yielded `end`.
#[lang = "RangeInclusive"]
#[derive(Clone, PartialEq, Eq)]
pub struct RangeInclusive<Idx> {
    pub start: Idx,
    pub end: Idx,
    pub exhausted: bool,
}

#[lang = "RangeToInclusive"]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RangeToInclusive<Idx> {
    pub end: Idx,
}

impl<Idx: PartialOrd<Idx>> Range<Idx> {
    pub fn contains(&self, item: &Idx) -> bool {
        *item >= self.start && *item < self.end
    }
    pub fn is_empty(&self) -> bool {
        !(self.start < self.end)
    }
}

impl<Idx: PartialOrd<Idx>> RangeFrom<Idx> {
    pub fn contains(&self, item: &Idx) -> bool {
        *item >= self.start
    }
}

impl<Idx: PartialOrd<Idx>> RangeTo<Idx> {
    pub fn contains(&self, item: &Idx) -> bool {
        *item < self.end
    }
}

impl<Idx: PartialOrd<Idx>> RangeToInclusive<Idx> {
    pub fn contains(&self, item: &Idx) -> bool {
        *item <= self.end
    }
}

impl<Idx> RangeInclusive<Idx> {
    pub const fn new(start: Idx, end: Idx) -> RangeInclusive<Idx> {
        RangeInclusive { start, end, exhausted: false }
    }
    pub fn start(&self) -> &Idx {
        &self.start
    }
    pub fn end(&self) -> &Idx {
        &self.end
    }
    pub fn into_inner(self) -> (Idx, Idx) {
        (self.start, self.end)
    }
}

impl<Idx: PartialOrd<Idx>> RangeInclusive<Idx> {
    pub fn contains(&self, item: &Idx) -> bool {
        *item >= self.start && *item <= self.end
    }
    pub fn is_empty(&self) -> bool {
        self.exhausted || !(self.start <= self.end)
    }
}

impl<Idx: crate::fmt::Debug> crate::fmt::Debug for RangeInclusive<Idx> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        self.start.fmt(f)?;
        f.write_str("..=")?;
        self.end.fmt(f)
    }
}

pub enum Bound<T> {
    Included(T),
    Excluded(T),
    Unbounded,
}

pub trait RangeBounds<T: ?Sized> {
    fn start_bound(&self) -> Bound<&T>;
    fn end_bound(&self) -> Bound<&T>;
}

impl<T> RangeBounds<T> for RangeFull {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
}
impl<T> RangeBounds<T> for Range<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(&self.start)
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Excluded(&self.end)
    }
}
impl<T> RangeBounds<T> for RangeFrom<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(&self.start)
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
}
impl<T> RangeBounds<T> for RangeTo<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Excluded(&self.end)
    }
}
impl<T> RangeBounds<T> for RangeInclusive<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(&self.start)
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Included(&self.end)
    }
}
impl<T> RangeBounds<T> for RangeToInclusive<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
    fn end_bound(&self) -> Bound<&T> {
        Bound::Included(&self.end)
    }
}

/// Resolve any `RangeBounds<usize>` against a length: (start, end) with
/// start <= end <= len, panicking like slice indexing does.
pub fn range_to_bounds<R: RangeBounds<usize>>(r: &R, len: usize) -> (usize, usize) {
    let start = match r.start_bound() {
        Bound::Included(s) => *s,
        Bound::Excluded(s) => *s + 1,
        Bound::Unbounded => 0,
    };
    let end = match r.end_bound() {
        Bound::Included(e) => *e + 1,
        Bound::Excluded(e) => *e,
        Bound::Unbounded => len,
    };
    if start > end {
        panic!("slice index starts at {} but ends at {}", start, end);
    }
    if end > len {
        panic!("range end index {} out of range for slice of length {}", end, len);
    }
    (start, end)
}

// Debug for the remaining range types, and the common Bound impls
// (added with std::collections).
impl<Idx: crate::fmt::Debug> crate::fmt::Debug for Range<Idx> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        self.start.fmt(f)?;
        f.write_str("..")?;
        self.end.fmt(f)
    }
}
impl<Idx: crate::fmt::Debug> crate::fmt::Debug for RangeFrom<Idx> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        self.start.fmt(f)?;
        f.write_str("..")
    }
}
impl<Idx: crate::fmt::Debug> crate::fmt::Debug for RangeTo<Idx> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("..")?;
        self.end.fmt(f)
    }
}
impl<Idx: crate::fmt::Debug> crate::fmt::Debug for RangeToInclusive<Idx> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("..=")?;
        self.end.fmt(f)
    }
}
impl crate::fmt::Debug for RangeFull {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("..")
    }
}
impl<T: Clone> Clone for Bound<T> {
    fn clone(&self) -> Bound<T> {
        match self {
            Bound::Included(x) => Bound::Included(x.clone()),
            Bound::Excluded(x) => Bound::Excluded(x.clone()),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}
impl<T: Copy> Copy for Bound<T> {}
impl<T: PartialEq> PartialEq for Bound<T> {
    fn eq(&self, other: &Bound<T>) -> bool {
        match (self, other) {
            (Bound::Included(a), Bound::Included(b)) => *a == *b,
            (Bound::Excluded(a), Bound::Excluded(b)) => *a == *b,
            (Bound::Unbounded, Bound::Unbounded) => true,
            _ => false,
        }
    }
}
impl<T: Eq> Eq for Bound<T> {}
impl<T: crate::fmt::Debug> crate::fmt::Debug for Bound<T> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        match self {
            Bound::Included(x) => f.debug_tuple("Included").field(x).finish(),
            Bound::Excluded(x) => f.debug_tuple("Excluded").field(x).finish(),
            Bound::Unbounded => f.write_str("Unbounded"),
        }
    }
}
