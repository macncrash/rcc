//! `std::collections`: HashMap/HashSet (insertion-ordered open addressing,
//! honouring the `BuildHasher`), BTreeMap/BTreeSet (chunked sorted vectors),
//! VecDeque (ring buffer) and BinaryHeap. See docs/STATUS.md
//! ("std: collections").

pub mod hash_map;
pub mod hash_set;
pub mod btree_map;
pub mod btree_set;
pub mod vec_deque;
pub mod binary_heap;

pub use self::hash_map::HashMap;
pub use self::hash_set::HashSet;
pub use self::btree_map::BTreeMap;
pub use self::btree_set::BTreeSet;
pub use self::vec_deque::VecDeque;
pub use self::binary_heap::BinaryHeap;

/// Real std has these as `std::collections::{btree_map, ..}::..`; the
/// `hash` path matches `std::collections::hash_map::DefaultHasher`.
pub use crate::hash::{DefaultHasher, RandomState};

/// Allocation failure from `try_reserve` (never produced: allocation aborts).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TryReserveError {
    kind: TryReserveErrorKind,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TryReserveErrorKind {
    CapacityOverflow,
    AllocError,
}

impl TryReserveError {
    pub fn kind(&self) -> TryReserveErrorKind {
        self.kind.clone()
    }
}

impl crate::fmt::Display for TryReserveError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("memory allocation failed")
    }
}

// `(Bound<T>, Bound<T>)` ranges (real std has these in core::ops::range;
// they live here because only the ordered collections use them).
use crate::ops::{Bound, RangeBounds};

impl<T> RangeBounds<T> for (Bound<T>, Bound<T>) {
    fn start_bound(&self) -> Bound<&T> {
        match &self.0 {
            Bound::Included(x) => Bound::Included(x),
            Bound::Excluded(x) => Bound::Excluded(x),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
    fn end_bound(&self) -> Bound<&T> {
        match &self.1 {
            Bound::Included(x) => Bound::Included(x),
            Bound::Excluded(x) => Bound::Excluded(x),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}

impl<'a, T: ?Sized + 'a> RangeBounds<T> for (Bound<&'a T>, Bound<&'a T>) {
    fn start_bound(&self) -> Bound<&T> {
        match &self.0 {
            Bound::Included(x) => Bound::Included(*x),
            Bound::Excluded(x) => Bound::Excluded(*x),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
    fn end_bound(&self) -> Bound<&T> {
        match &self.1 {
            Bound::Included(x) => Bound::Included(*x),
            Bound::Excluded(x) => Bound::Excluded(*x),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}
