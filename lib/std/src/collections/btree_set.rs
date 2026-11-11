//! `BTreeSet<T>`: a `BTreeMap<T, ()>`. Set operations are ordered merges.

use crate::borrow::Borrow;
use crate::cmp::Ordering;
use crate::collections::btree_map::{self, BTreeMap};
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::iter::{FromIterator, FusedIterator};
use crate::ops::{BitAnd, BitOr, BitXor, RangeBounds, Sub};

pub struct BTreeSet<T> {
    map: BTreeMap<T, ()>,
}

impl<T> BTreeSet<T> {
    pub const fn new() -> BTreeSet<T> {
        BTreeSet { map: BTreeMap::new() }
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    pub fn clear(&mut self) {
        self.map.clear()
    }
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { inner: self.map.keys() }
    }
    pub fn first(&self) -> Option<&T> {
        match self.map.first_key_value() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    pub fn last(&self) -> Option<&T> {
        match self.map.last_key_value() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    pub fn pop_first(&mut self) -> Option<T> {
        match self.map.pop_first() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    pub fn pop_last(&mut self) -> Option<T> {
        match self.map.pop_last() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut f: F) {
        self.map.retain(|k, _| f(k))
    }
}

impl<T: Ord> BTreeSet<T> {
    pub fn insert(&mut self, value: T) -> bool {
        match self.map.entry(value) {
            btree_map::Entry::Occupied(_) => false,
            btree_map::Entry::Vacant(e) => {
                e.insert(());
                true
            }
        }
    }
    pub fn replace(&mut self, value: T) -> Option<T> {
        let old = self.map.remove_entry(&value);
        self.map.insert(value, ());
        match old {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    pub fn contains<Q: ?Sized + Ord>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
    {
        self.map.contains_key(value)
    }
    pub fn get<Q: ?Sized + Ord>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
    {
        match self.map.get_key_value(value) {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    pub fn remove<Q: ?Sized + Ord>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
    {
        self.map.remove(value).is_some()
    }
    pub fn take<Q: ?Sized + Ord>(&mut self, value: &Q) -> Option<T>
    where
        T: Borrow<Q>,
    {
        match self.map.remove_entry(value) {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    pub fn range<K: ?Sized + Ord, R: RangeBounds<K>>(&self, range: R) -> Range<'_, T>
    where
        T: Borrow<K>,
    {
        Range { inner: self.map.range(range) }
    }
    pub fn append(&mut self, other: &mut BTreeSet<T>) {
        self.map.append(&mut other.map)
    }
    pub fn split_off<Q: ?Sized + Ord>(&mut self, value: &Q) -> BTreeSet<T>
    where
        T: Borrow<Q>,
    {
        BTreeSet { map: self.map.split_off(value) }
    }
    pub fn extract_if<F: FnMut(&T) -> bool>(&mut self, mut pred: F) -> IntoIter<T> {
        IntoIter { inner: self.map.extract_if(|k, _| pred(k)) }
    }
    pub fn union<'a>(&'a self, other: &'a BTreeSet<T>) -> Union<'a, T> {
        Union { m: Merge::new(self.iter(), other.iter()) }
    }
    pub fn intersection<'a>(&'a self, other: &'a BTreeSet<T>) -> Intersection<'a, T> {
        Intersection { m: Merge::new(self.iter(), other.iter()) }
    }
    pub fn difference<'a>(&'a self, other: &'a BTreeSet<T>) -> Difference<'a, T> {
        Difference { m: Merge::new(self.iter(), other.iter()) }
    }
    pub fn symmetric_difference<'a>(&'a self, other: &'a BTreeSet<T>) -> SymmetricDifference<'a, T> {
        SymmetricDifference { m: Merge::new(self.iter(), other.iter()) }
    }
    pub fn is_disjoint(&self, other: &BTreeSet<T>) -> bool {
        self.intersection(other).next().is_none()
    }
    pub fn is_subset(&self, other: &BTreeSet<T>) -> bool {
        if self.len() > other.len() {
            return false;
        }
        self.difference(other).next().is_none()
    }
    pub fn is_superset(&self, other: &BTreeSet<T>) -> bool {
        other.is_subset(self)
    }
}

impl<T: Clone> Clone for BTreeSet<T> {
    fn clone(&self) -> BTreeSet<T> {
        BTreeSet { map: self.map.clone() }
    }
}
impl<T> Default for BTreeSet<T> {
    fn default() -> BTreeSet<T> {
        BTreeSet::new()
    }
}
impl<T: PartialEq> PartialEq for BTreeSet<T> {
    fn eq(&self, other: &BTreeSet<T>) -> bool {
        self.map == other.map
    }
}
impl<T: Eq> Eq for BTreeSet<T> {}
impl<T: PartialOrd> PartialOrd for BTreeSet<T> {
    fn partial_cmp(&self, other: &BTreeSet<T>) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}
impl<T: Ord> Ord for BTreeSet<T> {
    fn cmp(&self, other: &BTreeSet<T>) -> Ordering {
        self.iter().cmp(other.iter())
    }
}
impl<T: Hash> Hash for BTreeSet<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_length_prefix(self.len());
        for elt in self.iter() {
            elt.hash(state);
        }
    }
}
impl<T: fmt::Debug> fmt::Debug for BTreeSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T: Ord> Extend<T> for BTreeSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for x in iter {
            self.insert(x);
        }
    }
}
impl<'a, T: 'a + Ord + Copy> Extend<&'a T> for BTreeSet<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        for x in iter {
            self.insert(*x);
        }
    }
}
impl<T: Ord> FromIterator<T> for BTreeSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> BTreeSet<T> {
        let mut v: Vec<(T, ())> = Vec::new();
        for x in iter {
            v.push((x, ()));
        }
        BTreeSet { map: BTreeMap::from_iter(v) }
    }
}
impl<T: Ord, const N: usize> From<[T; N]> for BTreeSet<T> {
    fn from(arr: [T; N]) -> BTreeSet<T> {
        BTreeSet::from_iter(arr)
    }
}

impl<'a, T> IntoIterator for &'a BTreeSet<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}
impl<T> IntoIterator for BTreeSet<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { inner: self.map.into_iter() }
    }
}

impl<T: Ord + Clone> BitOr<&BTreeSet<T>> for &BTreeSet<T> {
    type Output = BTreeSet<T>;
    fn bitor(self, rhs: &BTreeSet<T>) -> BTreeSet<T> {
        self.union(rhs).cloned().collect()
    }
}
impl<T: Ord + Clone> BitAnd<&BTreeSet<T>> for &BTreeSet<T> {
    type Output = BTreeSet<T>;
    fn bitand(self, rhs: &BTreeSet<T>) -> BTreeSet<T> {
        self.intersection(rhs).cloned().collect()
    }
}
impl<T: Ord + Clone> BitXor<&BTreeSet<T>> for &BTreeSet<T> {
    type Output = BTreeSet<T>;
    fn bitxor(self, rhs: &BTreeSet<T>) -> BTreeSet<T> {
        self.symmetric_difference(rhs).cloned().collect()
    }
}
impl<T: Ord + Clone> Sub<&BTreeSet<T>> for &BTreeSet<T> {
    type Output = BTreeSet<T>;
    fn sub(self, rhs: &BTreeSet<T>) -> BTreeSet<T> {
        self.difference(rhs).cloned().collect()
    }
}

// ---- iterators -------------------------------------------------------------

pub struct Iter<'a, T> {
    inner: btree_map::Keys<'a, T, ()>,
}
impl<'a, T> Clone for Iter<'a, T> {
    fn clone(&self) -> Iter<'a, T> {
        Iter { inner: self.inner.clone() }
    }
}
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    fn count(self) -> usize {
        self.inner.len()
    }
    fn last(mut self) -> Option<&'a T> {
        self.inner.next_back()
    }
}
impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        self.inner.next_back()
    }
}
impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}
impl<'a, T> FusedIterator for Iter<'a, T> {}
impl<'a, T: fmt::Debug> fmt::Debug for Iter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct IntoIter<T> {
    inner: btree_map::IntoIter<T, ()>,
}
impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self.inner.next() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        match self.inner.next_back() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
}
impl<T> ExactSizeIterator for IntoIter<T> {}
impl<T> FusedIterator for IntoIter<T> {}

pub struct Range<'a, T> {
    inner: btree_map::Range<'a, T, ()>,
}
impl<'a, T> Clone for Range<'a, T> {
    fn clone(&self) -> Range<'a, T> {
        Range { inner: self.inner.clone() }
    }
}
impl<'a, T> Iterator for Range<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        match self.inner.next() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    fn last(mut self) -> Option<&'a T> {
        self.next_back()
    }
}
impl<'a, T> DoubleEndedIterator for Range<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        match self.inner.next_back() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
}
impl<'a, T> FusedIterator for Range<'a, T> {}

/// Two ordered iterators with one element of lookahead each.
struct Merge<'a, T> {
    a: Iter<'a, T>,
    b: Iter<'a, T>,
    pa: Option<&'a T>,
    pb: Option<&'a T>,
}
impl<'a, T> Clone for Merge<'a, T> {
    fn clone(&self) -> Merge<'a, T> {
        Merge { a: self.a.clone(), b: self.b.clone(), pa: self.pa, pb: self.pb }
    }
}
impl<'a, T: Ord> Merge<'a, T> {
    fn new(mut a: Iter<'a, T>, mut b: Iter<'a, T>) -> Merge<'a, T> {
        let pa = a.next();
        let pb = b.next();
        Merge { a, b, pa, pb }
    }
    /// Next element with its origin: (elem, in_a, in_b).
    fn step(&mut self) -> Option<(&'a T, bool, bool)> {
        match (self.pa, self.pb) {
            (None, None) => None,
            (Some(x), None) => {
                self.pa = self.a.next();
                Some((x, true, false))
            }
            (None, Some(y)) => {
                self.pb = self.b.next();
                Some((y, false, true))
            }
            (Some(x), Some(y)) => match Ord::cmp(x, y) {
                Ordering::Less => {
                    self.pa = self.a.next();
                    Some((x, true, false))
                }
                Ordering::Greater => {
                    self.pb = self.b.next();
                    Some((y, false, true))
                }
                Ordering::Equal => {
                    self.pa = self.a.next();
                    self.pb = self.b.next();
                    Some((x, true, true))
                }
            },
        }
    }
}

pub struct Union<'a, T> {
    m: Merge<'a, T>,
}
impl<'a, T> Clone for Union<'a, T> {
    fn clone(&self) -> Union<'a, T> {
        Union { m: self.m.clone() }
    }
}
impl<'a, T: Ord> Iterator for Union<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        match self.m.step() {
            Some((x, _, _)) => Some(x),
            None => None,
        }
    }
}
impl<'a, T: Ord> FusedIterator for Union<'a, T> {}
impl<'a, T: Ord + fmt::Debug> fmt::Debug for Union<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct Intersection<'a, T> {
    m: Merge<'a, T>,
}
impl<'a, T> Clone for Intersection<'a, T> {
    fn clone(&self) -> Intersection<'a, T> {
        Intersection { m: self.m.clone() }
    }
}
impl<'a, T: Ord> Iterator for Intersection<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        loop {
            if self.m.pa.is_none() || self.m.pb.is_none() {
                return None;
            }
            match self.m.step() {
                Some((x, true, true)) => return Some(x),
                Some(_) => {}
                None => return None,
            }
        }
    }
}
impl<'a, T: Ord> FusedIterator for Intersection<'a, T> {}
impl<'a, T: Ord + fmt::Debug> fmt::Debug for Intersection<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct Difference<'a, T> {
    m: Merge<'a, T>,
}
impl<'a, T> Clone for Difference<'a, T> {
    fn clone(&self) -> Difference<'a, T> {
        Difference { m: self.m.clone() }
    }
}
impl<'a, T: Ord> Iterator for Difference<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        loop {
            if self.m.pa.is_none() {
                return None;
            }
            match self.m.step() {
                Some((x, true, false)) => return Some(x),
                Some(_) => {}
                None => return None,
            }
        }
    }
}
impl<'a, T: Ord> FusedIterator for Difference<'a, T> {}
impl<'a, T: Ord + fmt::Debug> fmt::Debug for Difference<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct SymmetricDifference<'a, T> {
    m: Merge<'a, T>,
}
impl<'a, T> Clone for SymmetricDifference<'a, T> {
    fn clone(&self) -> SymmetricDifference<'a, T> {
        SymmetricDifference { m: self.m.clone() }
    }
}
impl<'a, T: Ord> Iterator for SymmetricDifference<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        loop {
            match self.m.step() {
                Some((_, true, true)) => {}
                Some((x, _, _)) => return Some(x),
                None => return None,
            }
        }
    }
}
impl<'a, T: Ord> FusedIterator for SymmetricDifference<'a, T> {}
impl<'a, T: Ord + fmt::Debug> fmt::Debug for SymmetricDifference<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}
