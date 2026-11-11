//! `HashSet<T, S>`: a `HashMap<T, (), S>`.

use crate::borrow::Borrow;
use crate::collections::hash_map::{self, HashMap, RandomState};
use crate::fmt;
use crate::hash::{BuildHasher, Hash};
use crate::iter::{Chain, FromIterator, FusedIterator};
use crate::ops::{BitAnd, BitOr, BitXor, Sub};

pub struct HashSet<T, S = RandomState> {
    map: HashMap<T, (), S>,
}

impl<T> HashSet<T, RandomState> {
    pub fn new() -> HashSet<T, RandomState> {
        HashSet { map: HashMap::new() }
    }
    pub fn with_capacity(capacity: usize) -> HashSet<T, RandomState> {
        HashSet { map: HashMap::with_capacity(capacity) }
    }
}

impl<T, S> HashSet<T, S> {
    pub const fn with_hasher(hasher: S) -> HashSet<T, S> {
        HashSet { map: HashMap::with_hasher(hasher) }
    }
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> HashSet<T, S> {
        HashSet { map: HashMap::with_capacity_and_hasher(capacity, hasher) }
    }
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }
    pub fn hasher(&self) -> &S {
        self.map.hasher()
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
    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain { inner: self.map.drain() }
    }
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut f: F) {
        self.map.retain(|k, _| f(k))
    }
    pub fn extract_if<F: FnMut(&T) -> bool>(&mut self, mut pred: F) -> IntoIter<T> {
        IntoIter { inner: self.map.extract_if(|k, _| pred(k)) }
    }
}

impl<T: Eq + Hash, S: BuildHasher> HashSet<T, S> {
    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional)
    }
    pub fn shrink_to_fit(&mut self) {
        self.map.shrink_to_fit()
    }
    pub fn insert(&mut self, value: T) -> bool {
        match self.map.entry(value) {
            hash_map::Entry::Occupied(_) => false,
            hash_map::Entry::Vacant(e) => {
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
    pub fn contains<Q: ?Sized + Hash + Eq>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
    {
        self.map.contains_key(value)
    }
    pub fn get<Q: ?Sized + Hash + Eq>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
    {
        match self.map.get_key_value(value) {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    pub fn get_or_insert(&mut self, value: T) -> &T {
        let e = self.map.entry(value);
        match e {
            hash_map::Entry::Occupied(o) => {
                let r: *const T = o.key();
                unsafe { &*r }
            }
            hash_map::Entry::Vacant(v) => {
                let o = v.insert_entry(());
                let r: *const T = o.key();
                unsafe { &*r }
            }
        }
    }
    pub fn remove<Q: ?Sized + Hash + Eq>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
    {
        self.map.remove(value).is_some()
    }
    pub fn take<Q: ?Sized + Hash + Eq>(&mut self, value: &Q) -> Option<T>
    where
        T: Borrow<Q>,
    {
        match self.map.remove_entry(value) {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    pub fn union<'a>(&'a self, other: &'a HashSet<T, S>) -> Union<'a, T, S> {
        Union { iter: self.iter().chain(other.difference(self)) }
    }
    pub fn intersection<'a>(&'a self, other: &'a HashSet<T, S>) -> Intersection<'a, T, S> {
        if self.len() <= other.len() {
            Intersection { iter: self.iter(), other }
        } else {
            Intersection { iter: other.iter(), other: self }
        }
    }
    pub fn difference<'a>(&'a self, other: &'a HashSet<T, S>) -> Difference<'a, T, S> {
        Difference { iter: self.iter(), other }
    }
    pub fn symmetric_difference<'a>(&'a self, other: &'a HashSet<T, S>) -> SymmetricDifference<'a, T, S> {
        SymmetricDifference { iter: self.difference(other).chain(other.difference(self)) }
    }
    pub fn is_disjoint(&self, other: &HashSet<T, S>) -> bool {
        if self.len() <= other.len() {
            self.iter().all(|v| !other.contains(v))
        } else {
            other.iter().all(|v| !self.contains(v))
        }
    }
    pub fn is_subset(&self, other: &HashSet<T, S>) -> bool {
        if self.len() <= other.len() { self.iter().all(|v| other.contains(v)) } else { false }
    }
    pub fn is_superset(&self, other: &HashSet<T, S>) -> bool {
        other.is_subset(self)
    }
}

impl<T: Clone, S: Clone> Clone for HashSet<T, S> {
    fn clone(&self) -> HashSet<T, S> {
        HashSet { map: self.map.clone() }
    }
}
impl<T, S: Default> Default for HashSet<T, S> {
    fn default() -> HashSet<T, S> {
        HashSet { map: HashMap::default() }
    }
}
impl<T: Eq + Hash, S: BuildHasher> PartialEq for HashSet<T, S> {
    fn eq(&self, other: &HashSet<T, S>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.iter().all(|key| other.contains(key))
    }
}
impl<T: Eq + Hash, S: BuildHasher> Eq for HashSet<T, S> {}

impl<T: fmt::Debug, S> fmt::Debug for HashSet<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T: Eq + Hash, S: BuildHasher> Extend<T> for HashSet<T, S> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for k in iter {
            self.insert(k);
        }
    }
}
impl<'a, T: 'a + Eq + Hash + Copy, S: BuildHasher> Extend<&'a T> for HashSet<T, S> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        for k in iter {
            self.insert(*k);
        }
    }
}
impl<T: Eq + Hash, S: BuildHasher + Default> FromIterator<T> for HashSet<T, S> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> HashSet<T, S> {
        let mut set = HashSet::with_hasher(S::default());
        set.extend(iter);
        set
    }
}
impl<T: Eq + Hash, const N: usize> From<[T; N]> for HashSet<T, RandomState> {
    fn from(arr: [T; N]) -> HashSet<T, RandomState> {
        let mut set = HashSet::new();
        set.extend(arr);
        set
    }
}

impl<'a, T, S> IntoIterator for &'a HashSet<T, S> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}
impl<T, S> IntoIterator for HashSet<T, S> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { inner: self.map.into_iter() }
    }
}

impl<T: Eq + Hash + Clone, S: BuildHasher + Default> BitOr<&HashSet<T, S>> for &HashSet<T, S> {
    type Output = HashSet<T, S>;
    fn bitor(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        self.union(rhs).cloned().collect()
    }
}
impl<T: Eq + Hash + Clone, S: BuildHasher + Default> BitAnd<&HashSet<T, S>> for &HashSet<T, S> {
    type Output = HashSet<T, S>;
    fn bitand(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        self.intersection(rhs).cloned().collect()
    }
}
impl<T: Eq + Hash + Clone, S: BuildHasher + Default> BitXor<&HashSet<T, S>> for &HashSet<T, S> {
    type Output = HashSet<T, S>;
    fn bitxor(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        self.symmetric_difference(rhs).cloned().collect()
    }
}
impl<T: Eq + Hash + Clone, S: BuildHasher + Default> Sub<&HashSet<T, S>> for &HashSet<T, S> {
    type Output = HashSet<T, S>;
    fn sub(self, rhs: &HashSet<T, S>) -> HashSet<T, S> {
        self.difference(rhs).cloned().collect()
    }
}

// ---- iterators -------------------------------------------------------------

pub struct Iter<'a, K> {
    inner: hash_map::Keys<'a, K, ()>,
}
impl<'a, K> Clone for Iter<'a, K> {
    fn clone(&self) -> Iter<'a, K> {
        Iter { inner: self.inner.clone() }
    }
}
impl<'a, K> Iterator for Iter<'a, K> {
    type Item = &'a K;
    #[inline]
    fn next(&mut self) -> Option<&'a K> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    fn count(self) -> usize {
        self.inner.len()
    }
}
impl<'a, K> ExactSizeIterator for Iter<'a, K> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}
impl<'a, K> FusedIterator for Iter<'a, K> {}
impl<'a, K: fmt::Debug> fmt::Debug for Iter<'a, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct IntoIter<K> {
    inner: hash_map::IntoIter<K, ()>,
}
impl<K> Iterator for IntoIter<K> {
    type Item = K;
    #[inline]
    fn next(&mut self) -> Option<K> {
        match self.inner.next() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<K> ExactSizeIterator for IntoIter<K> {}
impl<K> FusedIterator for IntoIter<K> {}

pub struct Drain<'a, K> {
    inner: hash_map::Drain<'a, K, ()>,
}
impl<'a, K> Iterator for Drain<'a, K> {
    type Item = K;
    #[inline]
    fn next(&mut self) -> Option<K> {
        match self.inner.next() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<'a, K> ExactSizeIterator for Drain<'a, K> {}
impl<'a, K> FusedIterator for Drain<'a, K> {}

pub struct Intersection<'a, T, S> {
    iter: Iter<'a, T>,
    other: &'a HashSet<T, S>,
}
impl<'a, T, S> Clone for Intersection<'a, T, S> {
    fn clone(&self) -> Intersection<'a, T, S> {
        Intersection { iter: self.iter.clone(), other: self.other }
    }
}
impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for Intersection<'a, T, S> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        loop {
            match self.iter.next() {
                None => return None,
                Some(elt) => {
                    if self.other.contains(elt) {
                        return Some(elt);
                    }
                }
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}
impl<'a, T: Eq + Hash, S: BuildHasher> FusedIterator for Intersection<'a, T, S> {}
impl<'a, T: fmt::Debug + Eq + Hash, S: BuildHasher> fmt::Debug for Intersection<'a, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct Difference<'a, T, S> {
    iter: Iter<'a, T>,
    other: &'a HashSet<T, S>,
}
impl<'a, T, S> Clone for Difference<'a, T, S> {
    fn clone(&self) -> Difference<'a, T, S> {
        Difference { iter: self.iter.clone(), other: self.other }
    }
}
impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for Difference<'a, T, S> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        loop {
            match self.iter.next() {
                None => return None,
                Some(elt) => {
                    if !self.other.contains(elt) {
                        return Some(elt);
                    }
                }
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}
impl<'a, T: Eq + Hash, S: BuildHasher> FusedIterator for Difference<'a, T, S> {}
impl<'a, T: fmt::Debug + Eq + Hash, S: BuildHasher> fmt::Debug for Difference<'a, T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct SymmetricDifference<'a, T, S> {
    iter: Chain<Difference<'a, T, S>, Difference<'a, T, S>>,
}
impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for SymmetricDifference<'a, T, S> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<'a, T: Eq + Hash, S: BuildHasher> FusedIterator for SymmetricDifference<'a, T, S> {}

pub struct Union<'a, T, S> {
    iter: Chain<Iter<'a, T>, Difference<'a, T, S>>,
}
impl<'a, T: Eq + Hash, S: BuildHasher> Iterator for Union<'a, T, S> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<'a, T: Eq + Hash, S: BuildHasher> FusedIterator for Union<'a, T, S> {}
