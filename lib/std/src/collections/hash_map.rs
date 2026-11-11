//! `HashMap<K, V, S>`: an insertion-ordered hash map ("index map" layout).
//!
//! Entries live densely in a `Vec<Bucket>`; a power-of-two open-addressing
//! table of `u64` slots maps hashes to entry indices with linear probing.
//! A slot is 0 when empty, otherwise `(h32 << 32) | (index + 1)` where `h32`
//! is a 32-bit mix of the full hash (so most mismatches are rejected without
//! touching the entry). Removal uses backward-shift deletion (no tombstones)
//! and `swap_remove` on the entry vector, so iteration order is insertion
//! order until the first removal; it is deterministic in every case.
//! The table is kept at most 70% full.

use crate::borrow::Borrow;
use crate::fmt;
use crate::hash::{BuildHasher, Hash, Hasher};
use crate::iter::{FromIterator, FusedIterator};
use crate::marker::PhantomData;
use crate::mem;
use crate::ops::Index;

pub use crate::hash::{DefaultHasher, RandomState};

pub(crate) struct Bucket<K, V> {
    pub(crate) key: K,
    pub(crate) value: V,
    hash: u32,
}

impl<K: Clone, V: Clone> Clone for Bucket<K, V> {
    fn clone(&self) -> Bucket<K, V> {
        Bucket { key: self.key.clone(), value: self.value.clone(), hash: self.hash }
    }
}

/// The hasher-independent part of the map.
pub(crate) struct RawMap<K, V> {
    entries: Vec<Bucket<K, V>>,
    table: Vec<u64>,
    mask: usize,
    shift: u32,
}

#[inline]
fn mix_hash(h: u64) -> u32 {
    (h.wrapping_mul(0x9E3779B97F4A7C15) >> 32) as u32
}

#[inline]
fn make_slot(h32: u32, idx: usize) -> u64 {
    ((h32 as u64) << 32) | ((idx as u64) + 1)
}

#[inline]
fn slot_index(s: u64) -> usize {
    ((s & 0xffff_ffff) as usize) - 1
}

/// Smallest table size (power of two, >= 8) that holds `n` entries.
fn table_size_for(n: usize) -> usize {
    let mut t = 8usize;
    while t * 7 / 10 < n {
        t *= 2;
    }
    t
}

pub(crate) fn make_hash<Q: ?Sized + Hash, S: BuildHasher>(s: &S, q: &Q) -> u64 {
    let mut h = s.build_hasher();
    q.hash(&mut h);
    h.finish()
}

impl<K, V> RawMap<K, V> {
    pub(crate) const fn new() -> RawMap<K, V> {
        RawMap { entries: Vec::new(), table: Vec::new(), mask: 0, shift: 0 }
    }
    pub(crate) fn with_capacity(n: usize) -> RawMap<K, V> {
        let mut m = RawMap { entries: Vec::with_capacity(n), table: Vec::new(), mask: 0, shift: 0 };
        if n > 0 {
            m.rebuild(table_size_for(n));
        }
        m
    }
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
    pub(crate) fn capacity(&self) -> usize {
        let t = self.table.len() * 7 / 10;
        if t > self.entries.capacity() { t } else { self.entries.capacity() }
    }
    #[inline]
    fn home(&self, h32: u32) -> usize {
        (h32 >> self.shift) as usize
    }
    /// Rebuild the table with `size` slots from the entries.
    fn rebuild(&mut self, size: usize) {
        let mut bits = 0u32;
        while (1usize << bits) < size {
            bits += 1;
        }
        self.table = crate::vec::from_elem(0u64, size);
        self.mask = size - 1;
        self.shift = 32 - bits;
        let n = self.entries.len();
        let mut i = 0;
        while i < n {
            let h32 = self.entries[i].hash;
            self.place(h32, i);
            i += 1;
        }
    }
    /// Put index `idx` into the first free slot of its probe sequence.
    #[inline]
    fn place(&mut self, h32: u32, idx: usize) {
        let mut pos = self.home(h32);
        while self.table[pos] != 0 {
            pos = (pos + 1) & self.mask;
        }
        self.table[pos] = make_slot(h32, idx);
    }
    pub(crate) fn reserve(&mut self, additional: usize) {
        let need = self.entries.len() + additional;
        if need * 10 > self.table.len() * 7 {
            self.rebuild(table_size_for(need));
        }
        if need > self.entries.capacity() {
            self.entries.reserve(additional);
        }
    }
    pub(crate) fn shrink_to_fit(&mut self) {
        self.entries.shrink_to_fit();
        if self.entries.is_empty() {
            self.table = Vec::new();
            self.mask = 0;
            self.shift = 0;
        } else {
            let t = table_size_for(self.entries.len());
            if t < self.table.len() {
                self.rebuild(t);
            }
        }
    }
    /// Find the entry with hash `h32` for which `eq` holds: (slot, index).
    #[inline]
    pub(crate) fn find<F: FnMut(&K) -> bool>(&self, h32: u32, mut eq: F) -> Option<(usize, usize)> {
        if self.table.is_empty() {
            return None;
        }
        let tag = (h32 as u64) << 32;
        let mut pos = self.home(h32);
        loop {
            let s = self.table[pos];
            if s == 0 {
                return None;
            }
            if (s & 0xffff_ffff_0000_0000) == tag {
                let idx = slot_index(s);
                if eq(&self.entries[idx].key) {
                    return Some((pos, idx));
                }
            }
            pos = (pos + 1) & self.mask;
        }
    }
    /// Append a new entry (the key must not be present); returns its index.
    pub(crate) fn push(&mut self, h32: u32, key: K, value: V) -> usize {
        self.reserve(1);
        let idx = self.entries.len();
        self.entries.push(Bucket { key, value, hash: h32 });
        self.place(h32, idx);
        idx
    }
    /// Remove the entry at (slot, index).
    pub(crate) fn remove_at(&mut self, pos: usize, idx: usize) -> (K, V) {
        // backward-shift deletion of the slot
        let mask = self.mask;
        let mut hole = pos;
        let mut j = pos;
        loop {
            j = (j + 1) & mask;
            let s = self.table[j];
            if s == 0 {
                break;
            }
            let h = self.home((s >> 32) as u32);
            if (j.wrapping_sub(h) & mask) >= (j.wrapping_sub(hole) & mask) {
                self.table[hole] = s;
                hole = j;
            }
        }
        self.table[hole] = 0;
        // remove the entry; the last entry moves into its place
        let last = self.entries.len() - 1;
        let b = self.entries.swap_remove(idx);
        if idx != last {
            let h32 = self.entries[idx].hash;
            let old = make_slot(h32, last);
            let mut p = self.home(h32);
            while self.table[p] != old {
                p = (p + 1) & mask;
            }
            self.table[p] = make_slot(h32, idx);
        }
        (b.key, b.value)
    }
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        let n = self.table.len();
        let mut i = 0;
        while i < n {
            self.table[i] = 0;
            i += 1;
        }
    }
    pub(crate) fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut f: F) {
        let before = self.entries.len();
        self.entries.retain_mut(|b| f(&b.key, &mut b.value));
        if self.entries.len() != before {
            let n = self.table.len();
            let mut i = 0;
            while i < n {
                self.table[i] = 0;
                i += 1;
            }
            let m = self.entries.len();
            let mut k = 0;
            while k < m {
                let h32 = self.entries[k].hash;
                self.place(h32, k);
                k += 1;
            }
        }
    }
    pub(crate) fn take_entries(&mut self) -> Vec<Bucket<K, V>> {
        let v = mem::replace(&mut self.entries, Vec::new());
        self.clear();
        v
    }
    #[inline]
    pub(crate) fn entry_at(&self, idx: usize) -> &Bucket<K, V> {
        &self.entries[idx]
    }
    #[inline]
    pub(crate) fn entry_at_mut(&mut self, idx: usize) -> &mut Bucket<K, V> {
        &mut self.entries[idx]
    }
    pub(crate) fn into_entries(self) -> Vec<Bucket<K, V>> {
        self.entries
    }
}

impl<K: Clone, V: Clone> Clone for RawMap<K, V> {
    fn clone(&self) -> RawMap<K, V> {
        RawMap { entries: self.entries.clone(), table: self.table.clone(), mask: self.mask, shift: self.shift }
    }
}

// ---------------------------------------------------------------------------

pub struct HashMap<K, V, S = RandomState> {
    base: RawMap<K, V>,
    hash_builder: S,
}

impl<K, V> HashMap<K, V, RandomState> {
    pub fn new() -> HashMap<K, V, RandomState> {
        HashMap { base: RawMap::new(), hash_builder: RandomState::new() }
    }
    pub fn with_capacity(capacity: usize) -> HashMap<K, V, RandomState> {
        HashMap { base: RawMap::with_capacity(capacity), hash_builder: RandomState::new() }
    }
}

impl<K, V, S> HashMap<K, V, S> {
    pub const fn with_hasher(hash_builder: S) -> HashMap<K, V, S> {
        HashMap { base: RawMap::new(), hash_builder }
    }
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> HashMap<K, V, S> {
        HashMap { base: RawMap::with_capacity(capacity), hash_builder }
    }
    pub fn capacity(&self) -> usize {
        self.base.capacity()
    }
    pub fn hasher(&self) -> &S {
        &self.hash_builder
    }
    pub fn len(&self) -> usize {
        self.base.len()
    }
    pub fn is_empty(&self) -> bool {
        self.base.len() == 0
    }
    pub fn clear(&mut self) {
        self.base.clear()
    }
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter { inner: self.base.entries.iter() }
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut { inner: self.base.entries.iter_mut() }
    }
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { inner: self.base.entries.iter() }
    }
    pub fn values(&self) -> Values<'_, K, V> {
        Values { inner: self.base.entries.iter() }
    }
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut { inner: self.base.entries.iter_mut() }
    }
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys { inner: self.base.into_entries().into_iter() }
    }
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues { inner: self.base.into_entries().into_iter() }
    }
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        Drain { inner: self.base.take_entries().into_iter(), marker: PhantomData }
    }
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, f: F) {
        self.base.retain(f)
    }
    pub fn extract_if<F: FnMut(&K, &mut V) -> bool>(&mut self, mut pred: F) -> IntoIter<K, V> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.base.entries.len() {
            let b = &mut self.base.entries[i];
            if pred(&b.key, &mut b.value) {
                out.push(i);
            }
            i += 1;
        }
        let mut removed = Vec::with_capacity(out.len());
        let mut keep: Vec<Bucket<K, V>> = Vec::with_capacity(self.base.entries.len() - out.len());
        let all = self.base.take_entries();
        let mut j = 0;
        let mut k = 0;
        for b in all {
            if k < out.len() && out[k] == j {
                removed.push(b);
                k += 1;
            } else {
                keep.push(b);
            }
            j += 1;
        }
        self.base.entries = keep;
        let t = self.base.table.len();
        if t > 0 {
            self.base.rebuild(t);
        }
        IntoIter { inner: removed.into_iter() }
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> HashMap<K, V, S> {
    #[inline]
    fn hash_of<Q: ?Sized + Hash>(&self, q: &Q) -> u32 {
        mix_hash(make_hash(&self.hash_builder, q))
    }
    #[inline]
    fn find_key<Q: ?Sized + Hash + Eq>(&self, k: &Q) -> Option<(usize, usize)>
    where
        K: Borrow<Q>,
    {
        if self.base.len() == 0 {
            return None;
        }
        let h = self.hash_of(k);
        self.base.find(h, |x| x.borrow() == k)
    }
    pub fn reserve(&mut self, additional: usize) {
        self.base.reserve(additional)
    }
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.base.reserve(additional);
        Ok(())
    }
    pub fn shrink_to_fit(&mut self) {
        self.base.shrink_to_fit()
    }
    pub fn shrink_to(&mut self, _min_capacity: usize) {
        self.base.shrink_to_fit()
    }
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        let h = self.hash_of(&key);
        match self.base.find(h, |x| *x == key) {
            Some((pos, idx)) => Entry::Occupied(OccupiedEntry { map: &mut self.base, pos, idx, key: Some(key) }),
            None => Entry::Vacant(VacantEntry { map: &mut self.base, key, hash: h }),
        }
    }
    pub fn get<Q: ?Sized + Hash + Eq>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        match self.find_key(k) {
            Some((_, idx)) => Some(&self.base.entry_at(idx).value),
            None => None,
        }
    }
    pub fn get_key_value<Q: ?Sized + Hash + Eq>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
    {
        match self.find_key(k) {
            Some((_, idx)) => {
                let b = self.base.entry_at(idx);
                Some((&b.key, &b.value))
            }
            None => None,
        }
    }
    pub fn contains_key<Q: ?Sized + Hash + Eq>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.find_key(k).is_some()
    }
    pub fn get_mut<Q: ?Sized + Hash + Eq>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        match self.find_key(k) {
            Some((_, idx)) => Some(&mut self.base.entry_at_mut(idx).value),
            None => None,
        }
    }
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        let h = self.hash_of(&k);
        match self.base.find(h, |x| *x == k) {
            Some((_, idx)) => Some(mem::replace(&mut self.base.entry_at_mut(idx).value, v)),
            None => {
                self.base.push(h, k, v);
                None
            }
        }
    }
    pub fn try_insert(&mut self, k: K, v: V) -> Result<&mut V, V> {
        match self.entry(k) {
            Entry::Occupied(_) => Err(v),
            Entry::Vacant(e) => Ok(e.insert(v)),
        }
    }
    pub fn remove<Q: ?Sized + Hash + Eq>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        match self.find_key(k) {
            Some((pos, idx)) => Some(self.base.remove_at(pos, idx).1),
            None => None,
        }
    }
    pub fn remove_entry<Q: ?Sized + Hash + Eq>(&mut self, k: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
    {
        match self.find_key(k) {
            Some((pos, idx)) => Some(self.base.remove_at(pos, idx)),
            None => None,
        }
    }
}

impl<K: Clone, V: Clone, S: Clone> Clone for HashMap<K, V, S> {
    fn clone(&self) -> HashMap<K, V, S> {
        HashMap { base: self.base.clone(), hash_builder: self.hash_builder.clone() }
    }
}

impl<K, V, S: Default> Default for HashMap<K, V, S> {
    fn default() -> HashMap<K, V, S> {
        HashMap { base: RawMap::new(), hash_builder: S::default() }
    }
}

impl<K: Eq + Hash, V: PartialEq, S: BuildHasher> PartialEq for HashMap<K, V, S> {
    fn eq(&self, other: &HashMap<K, V, S>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for (k, v) in self.iter() {
            match other.get(k) {
                Some(w) => {
                    if !(*v == *w) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }
}
impl<K: Eq + Hash, V: Eq, S: BuildHasher> Eq for HashMap<K, V, S> {}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for HashMap<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, Q: ?Sized, V, S> Index<&Q> for HashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash,
    S: BuildHasher,
{
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        match self.get(key) {
            Some(v) => v,
            None => panic!("key not found in HashMap"),
        }
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> Extend<(K, V)> for HashMap<K, V, S> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        let it = iter.into_iter();
        let (lo, _) = it.size_hint();
        let add = if self.is_empty() { lo } else { (lo + 1) / 2 };
        self.reserve(add);
        for (k, v) in it {
            self.insert(k, v);
        }
    }
}
impl<'a, K: Eq + Hash + Copy, V: Copy, S: BuildHasher> Extend<(&'a K, &'a V)> for HashMap<K, V, S> {
    fn extend<T: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(*k, *v);
        }
    }
}

impl<K: Eq + Hash, V, S: BuildHasher + Default> FromIterator<(K, V)> for HashMap<K, V, S> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> HashMap<K, V, S> {
        let mut m = HashMap::with_hasher(S::default());
        m.extend(iter);
        m
    }
}

impl<K: Eq + Hash, V, const N: usize> From<[(K, V); N]> for HashMap<K, V, RandomState> {
    fn from(arr: [(K, V); N]) -> HashMap<K, V, RandomState> {
        let mut m = HashMap::new();
        m.extend(arr);
        m
    }
}

impl<'a, K, V, S> IntoIterator for &'a HashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}
impl<'a, K, V, S> IntoIterator for &'a mut HashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}
impl<K, V, S> IntoIterator for HashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter { inner: self.base.into_entries().into_iter() }
    }
}

// ---- iterators -------------------------------------------------------------

pub struct Iter<'a, K, V> {
    inner: crate::slice::Iter<'a, Bucket<K, V>>,
}
impl<'a, K, V> Clone for Iter<'a, K, V> {
    fn clone(&self) -> Iter<'a, K, V> {
        Iter { inner: self.inner.clone() }
    }
}
impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        match self.inner.next() {
            Some(b) => Some((&b.key, &b.value)),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    fn count(self) -> usize {
        self.inner.len()
    }
}
impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}
impl<'a, K, V> FusedIterator for Iter<'a, K, V> {}
impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for Iter<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct IterMut<'a, K, V> {
    inner: crate::slice::IterMut<'a, Bucket<K, V>>,
}
impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        match self.inner.next() {
            Some(b) => Some((&b.key, &mut b.value)),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}
impl<'a, K, V> FusedIterator for IterMut<'a, K, V> {}

pub struct Keys<'a, K, V> {
    inner: crate::slice::Iter<'a, Bucket<K, V>>,
}
impl<'a, K, V> Clone for Keys<'a, K, V> {
    fn clone(&self) -> Keys<'a, K, V> {
        Keys { inner: self.inner.clone() }
    }
}
impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;
    #[inline]
    fn next(&mut self) -> Option<&'a K> {
        match self.inner.next() {
            Some(b) => Some(&b.key),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    fn count(self) -> usize {
        self.inner.len()
    }
}
impl<'a, K, V> ExactSizeIterator for Keys<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}
impl<'a, K, V> FusedIterator for Keys<'a, K, V> {}
impl<'a, K: fmt::Debug, V> fmt::Debug for Keys<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct Values<'a, K, V> {
    inner: crate::slice::Iter<'a, Bucket<K, V>>,
}
impl<'a, K, V> Clone for Values<'a, K, V> {
    fn clone(&self) -> Values<'a, K, V> {
        Values { inner: self.inner.clone() }
    }
}
impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;
    #[inline]
    fn next(&mut self) -> Option<&'a V> {
        match self.inner.next() {
            Some(b) => Some(&b.value),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    fn count(self) -> usize {
        self.inner.len()
    }
}
impl<'a, K, V> ExactSizeIterator for Values<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}
impl<'a, K, V> FusedIterator for Values<'a, K, V> {}
impl<'a, K, V: fmt::Debug> fmt::Debug for Values<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct ValuesMut<'a, K, V> {
    inner: crate::slice::IterMut<'a, Bucket<K, V>>,
}
impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
    type Item = &'a mut V;
    #[inline]
    fn next(&mut self) -> Option<&'a mut V> {
        match self.inner.next() {
            Some(b) => Some(&mut b.value),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<'a, K, V> ExactSizeIterator for ValuesMut<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}
impl<'a, K, V> FusedIterator for ValuesMut<'a, K, V> {}

pub struct IntoIter<K, V> {
    inner: crate::vec::IntoIter<Bucket<K, V>>,
}
impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        match self.inner.next() {
            Some(b) => Some((b.key, b.value)),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<K, V> ExactSizeIterator for IntoIter<K, V> {}
impl<K, V> FusedIterator for IntoIter<K, V> {}

pub struct IntoKeys<K, V> {
    inner: crate::vec::IntoIter<Bucket<K, V>>,
}
impl<K, V> Iterator for IntoKeys<K, V> {
    type Item = K;
    #[inline]
    fn next(&mut self) -> Option<K> {
        match self.inner.next() {
            Some(b) => Some(b.key),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<K, V> ExactSizeIterator for IntoKeys<K, V> {}
impl<K, V> FusedIterator for IntoKeys<K, V> {}

pub struct IntoValues<K, V> {
    inner: crate::vec::IntoIter<Bucket<K, V>>,
}
impl<K, V> Iterator for IntoValues<K, V> {
    type Item = V;
    #[inline]
    fn next(&mut self) -> Option<V> {
        match self.inner.next() {
            Some(b) => Some(b.value),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<K, V> ExactSizeIterator for IntoValues<K, V> {}
impl<K, V> FusedIterator for IntoValues<K, V> {}

pub struct Drain<'a, K, V> {
    inner: crate::vec::IntoIter<Bucket<K, V>>,
    marker: PhantomData<&'a mut V>,
}
impl<'a, K, V> Iterator for Drain<'a, K, V> {
    type Item = (K, V);
    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        match self.inner.next() {
            Some(b) => Some((b.key, b.value)),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<'a, K, V> ExactSizeIterator for Drain<'a, K, V> {}
impl<'a, K, V> FusedIterator for Drain<'a, K, V> {}

// ---- entry API ---------------------------------------------------------------

pub enum Entry<'a, K, V> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

pub struct OccupiedEntry<'a, K, V> {
    map: &'a mut RawMap<K, V>,
    pos: usize,
    idx: usize,
    key: Option<K>,
}

pub struct VacantEntry<'a, K, V> {
    map: &'a mut RawMap<K, V>,
    key: K,
    hash: u32,
}

impl<'a, K, V> Entry<'a, K, V> {
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default),
        }
    }
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default()),
        }
    }
    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let v = default(&e.key);
                e.insert(v)
            }
        }
    }
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(e) => e.key(),
            Entry::Vacant(e) => e.key(),
        }
    }
    pub fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Entry<'a, K, V> {
        match self {
            Entry::Occupied(mut e) => {
                f(e.get_mut());
                Entry::Occupied(e)
            }
            Entry::Vacant(e) => Entry::Vacant(e),
        }
    }
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
        match self {
            Entry::Occupied(mut e) => {
                e.insert(value);
                e
            }
            Entry::Vacant(e) => e.insert_entry(value),
        }
    }
}

impl<'a, K, V: Default> Entry<'a, K, V> {
    pub fn or_default(self) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(V::default()),
        }
    }
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    pub fn key(&self) -> &K {
        &self.map.entry_at(self.idx).key
    }
    pub fn get(&self) -> &V {
        &self.map.entry_at(self.idx).value
    }
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.map.entry_at_mut(self.idx).value
    }
    pub fn into_mut(self) -> &'a mut V {
        &mut self.map.entry_at_mut(self.idx).value
    }
    pub fn insert(&mut self, value: V) -> V {
        mem::replace(&mut self.map.entry_at_mut(self.idx).value, value)
    }
    pub fn remove(self) -> V {
        self.map.remove_at(self.pos, self.idx).1
    }
    pub fn remove_entry(self) -> (K, V) {
        self.map.remove_at(self.pos, self.idx)
    }
    pub fn replace_entry(self, value: V) -> (K, V) {
        let b = self.map.entry_at_mut(self.idx);
        let k = match self.key {
            Some(k) => mem::replace(&mut b.key, k),
            None => panic!("replace_entry: no key"),
        };
        let v = mem::replace(&mut b.value, value);
        (k, v)
    }
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    pub fn key(&self) -> &K {
        &self.key
    }
    pub fn into_key(self) -> K {
        self.key
    }
    pub fn insert(self, value: V) -> &'a mut V {
        let idx = self.map.push(self.hash, self.key, value);
        &mut self.map.entry_at_mut(idx).value
    }
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
        let h = self.hash;
        let idx = self.map.push(h, self.key, value);
        let target = make_slot(h, idx);
        let mut pos = self.map.home(h);
        while self.map.table[pos] != target {
            pos = (pos + 1) & self.map.mask;
        }
        OccupiedEntry { map: self.map, pos, idx, key: None }
    }
}

impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for Entry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Entry::Occupied(e) => f.debug_tuple("Entry").field(e).finish(),
            Entry::Vacant(e) => f.debug_tuple("Entry").field(e).finish(),
        }
    }
}
impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for OccupiedEntry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OccupiedEntry").field("key", self.key()).field("value", self.get()).finish_non_exhaustive()
    }
}
impl<'a, K: fmt::Debug, V> fmt::Debug for VacantEntry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry").field(self.key()).finish()
    }
}
