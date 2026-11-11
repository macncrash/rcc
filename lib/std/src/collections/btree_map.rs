//! `BTreeMap<K, V>`: an ordered map stored as a sorted sequence of sorted
//! chunks (a two-level B+ tree). Each chunk holds 1..=CHUNK_MAX entries;
//! keys in chunk i are all smaller than those in chunk i+1. Lookups are two
//! binary searches; inserts/removes move at most CHUNK_MAX entries plus,
//! rarely (split/merge), the chunk headers. A position is a cursor
//! `(chunk, index)`, normalized so that `index < chunk.len()` except for the
//! end position `(chunks.len(), 0)`.

use crate::borrow::Borrow;
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::iter::{FromIterator, FusedIterator};
use crate::marker::PhantomData;
use crate::mem;
use crate::ops::{Bound, Index, RangeBounds};

const CHUNK_MAX: usize = 128;
const CHUNK_MIN: usize = 32;

pub struct BTreeMap<K, V> {
    chunks: Vec<Vec<(K, V)>>,
    length: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Pos {
    c: usize,
    i: usize,
}

/// First index in `chunk` whose key is not `< q` (or `<= q` with `after`).
#[inline]
fn chunk_bound<K: Borrow<Q>, V, Q: ?Sized + Ord>(chunk: &[(K, V)], q: &Q, after: bool) -> usize {
    let mut lo = 0usize;
    let mut hi = chunk.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let ord = Ord::cmp(chunk[mid].0.borrow(), q);
        let go_right = match ord {
            Ordering::Less => true,
            Ordering::Equal => after,
            Ordering::Greater => false,
        };
        if go_right {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

impl<K, V> BTreeMap<K, V> {
    pub const fn new() -> BTreeMap<K, V> {
        BTreeMap { chunks: Vec::new(), length: 0 }
    }
    pub fn len(&self) -> usize {
        self.length
    }
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.length = 0;
    }

    #[inline]
    fn end_pos(&self) -> Pos {
        Pos { c: self.chunks.len(), i: 0 }
    }
    #[inline]
    fn at(&self, p: Pos) -> &(K, V) {
        &self.chunks[p.c][p.i]
    }
    #[inline]
    fn at_mut(&mut self, p: Pos) -> &mut (K, V) {
        &mut self.chunks[p.c][p.i]
    }

    /// Cursor of the first key `>= q` (`> q` when `after`).
    fn bound<Q: ?Sized + Ord>(&self, q: &Q, after: bool) -> Pos
    where
        K: Borrow<Q>,
    {
        // first chunk whose last key is >= q (resp. > q)
        let mut lo = 0usize;
        let mut hi = self.chunks.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let ch = &self.chunks[mid];
            let ord = Ord::cmp(ch[ch.len() - 1].0.borrow(), q);
            let go_right = match ord {
                Ordering::Less => true,
                Ordering::Equal => after,
                Ordering::Greater => false,
            };
            if go_right {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo == self.chunks.len() {
            return self.end_pos();
        }
        Pos { c: lo, i: chunk_bound(&self.chunks[lo], q, after) }
    }

    /// Ok(position of key) or Err(insertion position).
    fn search<Q: ?Sized + Ord>(&self, q: &Q) -> Result<Pos, Pos>
    where
        K: Borrow<Q>,
    {
        let p = self.bound(q, false);
        if p.c < self.chunks.len() && Ord::cmp(self.at(p).0.borrow(), q) == Ordering::Equal {
            Ok(p)
        } else {
            Err(p)
        }
    }

    /// Insert at a position from `search` (Err); returns the final position.
    fn insert_at(&mut self, p: Pos, key: K, value: V) -> Pos {
        self.length += 1;
        if self.chunks.is_empty() {
            let mut v = Vec::with_capacity(8);
            v.push((key, value));
            self.chunks.push(v);
            return Pos { c: 0, i: 0 };
        }
        let mut c = p.c;
        let mut i = p.i;
        if c == self.chunks.len() {
            c -= 1;
            i = self.chunks[c].len();
        }
        self.chunks[c].insert(i, (key, value));
        if self.chunks[c].len() > CHUNK_MAX {
            let half = self.chunks[c].len() / 2;
            let tail = self.chunks[c].split_off(half);
            self.chunks.insert(c + 1, tail);
            if i >= half {
                c += 1;
                i -= half;
            }
        }
        Pos { c, i }
    }

    /// Remove the entry at a valid position.
    fn remove_at(&mut self, p: Pos) -> (K, V) {
        self.length -= 1;
        let kv = self.chunks[p.c].remove(p.i);
        let n = self.chunks[p.c].len();
        if n == 0 {
            self.chunks.remove(p.c);
        } else if n < CHUNK_MIN {
            // merge with a neighbour when the result stays small
            if p.c + 1 < self.chunks.len() && n + self.chunks[p.c + 1].len() <= CHUNK_MAX / 2 + CHUNK_MIN {
                let mut next = self.chunks.remove(p.c + 1);
                self.chunks[p.c].append(&mut next);
            } else if p.c > 0 && n + self.chunks[p.c - 1].len() <= CHUNK_MAX / 2 + CHUNK_MIN {
                let mut cur = self.chunks.remove(p.c);
                self.chunks[p.c - 1].append(&mut cur);
            }
        }
        kv
    }

    fn first_pos(&self) -> Pos {
        Pos { c: 0, i: 0 }
    }
    fn last_pos(&self) -> Pos {
        let c = self.chunks.len() - 1;
        Pos { c, i: self.chunks[c].len() - 1 }
    }

    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        if self.length == 0 {
            return None;
        }
        let kv = self.at(self.first_pos());
        Some((&kv.0, &kv.1))
    }
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        if self.length == 0 {
            return None;
        }
        let kv = self.at(self.last_pos());
        Some((&kv.0, &kv.1))
    }
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        if self.length == 0 {
            return None;
        }
        let p = self.first_pos();
        Some(self.remove_at(p))
    }
    pub fn pop_last(&mut self) -> Option<(K, V)> {
        if self.length == 0 {
            return None;
        }
        let p = self.last_pos();
        Some(self.remove_at(p))
    }
    pub fn first_entry(&mut self) -> Option<OccupiedEntry<'_, K, V>> {
        if self.length == 0 {
            return None;
        }
        let p = self.first_pos();
        Some(OccupiedEntry { map: self, pos: p })
    }
    pub fn last_entry(&mut self) -> Option<OccupiedEntry<'_, K, V>> {
        if self.length == 0 {
            return None;
        }
        let p = self.last_pos();
        Some(OccupiedEntry { map: self, pos: p })
    }

    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter { chunks: &self.chunks, front: Pos { c: 0, i: 0 }, back: self.end_pos(), remaining: self.length }
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        let end = self.end_pos();
        IterMut { raw: RawCursor::new(&mut self.chunks, Pos { c: 0, i: 0 }, end), remaining: self.length }
    }
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { inner: self.iter() }
    }
    pub fn values(&self) -> Values<'_, K, V> {
        Values { inner: self.iter() }
    }
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut { inner: self.iter_mut() }
    }
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys { inner: self.into_iter() }
    }
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues { inner: self.into_iter() }
    }

    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut f: F) {
        let mut removed = 0;
        let mut c = 0;
        while c < self.chunks.len() {
            let before = self.chunks[c].len();
            self.chunks[c].retain_mut(|kv| f(&kv.0, &mut kv.1));
            removed += before - self.chunks[c].len();
            c += 1;
        }
        if removed > 0 {
            self.length -= removed;
            self.rechunk();
        }
    }

    /// Drop empty chunks and merge runs of small ones.
    fn rechunk(&mut self) {
        let old = mem::replace(&mut self.chunks, Vec::new());
        for ch in old {
            if ch.is_empty() {
                continue;
            }
            let merge = match self.chunks.last() {
                Some(last) => last.len() + ch.len() <= CHUNK_MAX / 2 + CHUNK_MIN && (last.len() < CHUNK_MIN || ch.len() < CHUNK_MIN),
                None => false,
            };
            if merge {
                let n = self.chunks.len();
                let mut ch = ch;
                self.chunks[n - 1].append(&mut ch);
            } else {
                self.chunks.push(ch);
            }
        }
    }

    /// Flatten to one sorted vector.
    fn into_vec(self) -> Vec<(K, V)> {
        let mut out = Vec::with_capacity(self.length);
        for mut ch in self.chunks {
            out.append(&mut ch);
        }
        out
    }

    /// Build from a sorted, deduplicated vector.
    fn from_sorted_vec(v: Vec<(K, V)>) -> BTreeMap<K, V> {
        let length = v.len();
        let mut chunks = Vec::with_capacity(length / (CHUNK_MAX / 2) + 1);
        let mut cur: Vec<(K, V)> = Vec::with_capacity(CHUNK_MAX / 2);
        for kv in v {
            cur.push(kv);
            if cur.len() == CHUNK_MAX / 2 {
                chunks.push(mem::replace(&mut cur, Vec::with_capacity(CHUNK_MAX / 2)));
            }
        }
        if !cur.is_empty() {
            chunks.push(cur);
        }
        BTreeMap { chunks, length }
    }

    fn range_pos<T: ?Sized + Ord, R: RangeBounds<T>>(&self, range: &R) -> (Pos, Pos)
    where
        K: Borrow<T>,
    {
        let sb = range.start_bound();
        let eb = range.end_bound();
        match (&sb, &eb) {
            (Bound::Excluded(s), Bound::Excluded(e)) => {
                if Ord::cmp(*s, *e) == Ordering::Equal {
                    panic!("range start and end are equal and excluded in BTreeMap");
                }
            }
            _ => {}
        }
        match (&sb, &eb) {
            (Bound::Included(s), Bound::Included(e))
            | (Bound::Included(s), Bound::Excluded(e))
            | (Bound::Excluded(s), Bound::Included(e))
            | (Bound::Excluded(s), Bound::Excluded(e)) => {
                if Ord::cmp(*s, *e) == Ordering::Greater {
                    panic!("range start is greater than range end in BTreeMap");
                }
            }
            _ => {}
        }
        let front = match sb {
            Bound::Included(s) => self.bound(s, false),
            Bound::Excluded(s) => self.bound(s, true),
            Bound::Unbounded => Pos { c: 0, i: 0 },
        };
        let back = match eb {
            Bound::Included(e) => self.bound(e, true),
            Bound::Excluded(e) => self.bound(e, false),
            Bound::Unbounded => self.end_pos(),
        };
        // an empty range can produce front > back (e.g. (Excluded(x), Included(x)))
        if (front.c, front.i) > (back.c, back.i) {
            return (back, back);
        }
        (front, back)
    }
}

impl<K: Ord, V> BTreeMap<K, V> {
    pub fn get<Q: ?Sized + Ord>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        match self.search(key) {
            Ok(p) => Some(&self.at(p).1),
            Err(_) => None,
        }
    }
    pub fn get_key_value<Q: ?Sized + Ord>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
    {
        match self.search(key) {
            Ok(p) => {
                let kv = self.at(p);
                Some((&kv.0, &kv.1))
            }
            Err(_) => None,
        }
    }
    pub fn get_mut<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        match self.search(key) {
            Ok(p) => Some(&mut self.at_mut(p).1),
            Err(_) => None,
        }
    }
    pub fn contains_key<Q: ?Sized + Ord>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.search(key).is_ok()
    }
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.search(&key) {
            Ok(p) => Some(mem::replace(&mut self.at_mut(p).1, value)),
            Err(p) => {
                self.insert_at(p, key, value);
                None
            }
        }
    }
    pub fn try_insert(&mut self, key: K, value: V) -> Result<&mut V, V> {
        match self.entry(key) {
            Entry::Occupied(_) => Err(value),
            Entry::Vacant(e) => Ok(e.insert(value)),
        }
    }
    pub fn remove<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        match self.search(key) {
            Ok(p) => Some(self.remove_at(p).1),
            Err(_) => None,
        }
    }
    pub fn remove_entry<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
    {
        match self.search(key) {
            Ok(p) => Some(self.remove_at(p)),
            Err(_) => None,
        }
    }
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        match self.search(&key) {
            Ok(p) => Entry::Occupied(OccupiedEntry { map: self, pos: p }),
            Err(p) => Entry::Vacant(VacantEntry { map: self, key, pos: p }),
        }
    }
    pub fn range<T: ?Sized + Ord, R: RangeBounds<T>>(&self, range: R) -> Range<'_, K, V>
    where
        K: Borrow<T>,
    {
        let (front, back) = self.range_pos(&range);
        Range { chunks: &self.chunks, front, back }
    }
    pub fn range_mut<T: ?Sized + Ord, R: RangeBounds<T>>(&mut self, range: R) -> RangeMut<'_, K, V>
    where
        K: Borrow<T>,
    {
        let (front, back) = self.range_pos(&range);
        RangeMut { raw: RawCursor::new(&mut self.chunks, front, back) }
    }
    pub fn append(&mut self, other: &mut BTreeMap<K, V>) {
        if other.is_empty() {
            return;
        }
        if self.is_empty() {
            mem::swap(self, other);
            return;
        }
        let disjoint = Ord::cmp(&self.at(self.last_pos()).0, &other.at(other.first_pos()).0) == Ordering::Less;
        let o = mem::replace(other, BTreeMap::new());
        if disjoint {
            self.length += o.length;
            for ch in o.chunks {
                self.chunks.push(ch);
            }
        } else {
            for (k, v) in o {
                self.insert(k, v);
            }
        }
    }
    pub fn split_off<Q: ?Sized + Ord>(&mut self, key: &Q) -> BTreeMap<K, V>
    where
        K: Borrow<Q>,
    {
        let p = self.bound(key, false);
        if p.c == self.chunks.len() {
            return BTreeMap::new();
        }
        let mut right: Vec<Vec<(K, V)>> = Vec::new();
        let first = self.chunks[p.c].split_off(p.i);
        right.push(first);
        let rest = self.chunks.split_off(p.c + 1);
        for ch in rest {
            right.push(ch);
        }
        if self.chunks[p.c].is_empty() {
            self.chunks.pop();
        }
        let mut n = 0;
        for ch in right.iter() {
            n += ch.len();
        }
        self.length -= n;
        let mut out = BTreeMap { chunks: right, length: n };
        out.rechunk();
        self.rechunk();
        out
    }
    pub fn extract_if<F: FnMut(&K, &mut V) -> bool>(&mut self, mut pred: F) -> IntoIter<K, V> {
        let all = mem::replace(self, BTreeMap::new()).into_vec();
        let mut keep = Vec::new();
        let mut out = Vec::new();
        for mut kv in all {
            if pred(&kv.0, &mut kv.1) {
                out.push(kv);
            } else {
                keep.push(kv);
            }
        }
        *self = BTreeMap::from_sorted_vec(keep);
        IntoIter { inner: out.into_iter() }
    }
}

impl<K: Clone, V: Clone> Clone for BTreeMap<K, V> {
    fn clone(&self) -> BTreeMap<K, V> {
        BTreeMap { chunks: self.chunks.clone(), length: self.length }
    }
}
impl<K, V> Default for BTreeMap<K, V> {
    fn default() -> BTreeMap<K, V> {
        BTreeMap::new()
    }
}
impl<K: PartialEq, V: PartialEq> PartialEq for BTreeMap<K, V> {
    fn eq(&self, other: &BTreeMap<K, V>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut a = self.iter();
        let mut b = other.iter();
        loop {
            match (a.next(), b.next()) {
                (Some((k1, v1)), Some((k2, v2))) => {
                    if !(*k1 == *k2 && *v1 == *v2) {
                        return false;
                    }
                }
                _ => return true,
            }
        }
    }
}
impl<K: Eq, V: Eq> Eq for BTreeMap<K, V> {}
impl<K: PartialOrd, V: PartialOrd> PartialOrd for BTreeMap<K, V> {
    fn partial_cmp(&self, other: &BTreeMap<K, V>) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}
impl<K: Ord, V: Ord> Ord for BTreeMap<K, V> {
    fn cmp(&self, other: &BTreeMap<K, V>) -> Ordering {
        self.iter().cmp(other.iter())
    }
}
impl<K: Hash, V: Hash> Hash for BTreeMap<K, V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_length_prefix(self.len());
        for elt in self.iter() {
            elt.hash(state);
        }
    }
}
impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for BTreeMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}
impl<K: Ord + Borrow<Q>, Q: ?Sized + Ord, V> Index<&Q> for BTreeMap<K, V> {
    type Output = V;
    fn index(&self, key: &Q) -> &V {
        match self.get(key) {
            Some(v) => v,
            None => panic!("key not found"),
        }
    }
}

impl<K: Ord, V> Extend<(K, V)> for BTreeMap<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}
impl<'a, K: Ord + Copy, V: Copy> Extend<(&'a K, &'a V)> for BTreeMap<K, V> {
    fn extend<T: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(*k, *v);
        }
    }
}
impl<K: Ord, V> FromIterator<(K, V)> for BTreeMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> BTreeMap<K, V> {
        let mut v: Vec<(K, V)> = Vec::new();
        v.extend(iter);
        if v.is_empty() {
            return BTreeMap::new();
        }
        // stable sort, then keep the last value of each run of equal keys
        v.sort_by(|a, b| Ord::cmp(&a.0, &b.0));
        let mut out: Vec<(K, V)> = Vec::with_capacity(v.len());
        for kv in v {
            let dup = match out.last() {
                Some(last) => Ord::cmp(&last.0, &kv.0) == Ordering::Equal,
                None => false,
            };
            if dup {
                let n = out.len();
                out[n - 1] = kv;
            } else {
                out.push(kv);
            }
        }
        BTreeMap::from_sorted_vec(out)
    }
}
impl<K: Ord, V, const N: usize> From<[(K, V); N]> for BTreeMap<K, V> {
    fn from(arr: [(K, V); N]) -> BTreeMap<K, V> {
        BTreeMap::from_iter(arr)
    }
}

impl<'a, K, V> IntoIterator for &'a BTreeMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}
impl<'a, K, V> IntoIterator for &'a mut BTreeMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}
impl<K, V> IntoIterator for BTreeMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter { inner: self.into_vec().into_iter() }
    }
}

// ---- cursors and iterators -----------------------------------------------------

#[inline]
fn advance<K, V>(chunks: &[Vec<(K, V)>], p: &mut Pos) {
    p.i += 1;
    if p.i == chunks[p.c].len() {
        p.c += 1;
        p.i = 0;
    }
}
#[inline]
fn retreat<K, V>(chunks: &[Vec<(K, V)>], p: &mut Pos) {
    if p.i == 0 {
        p.c -= 1;
        p.i = chunks[p.c].len();
    }
    p.i -= 1;
}

/// Shared by the mutable iterators: a raw pointer to the chunk vector.
struct RawCursor<'a, K, V> {
    chunks: *mut Vec<(K, V)>,
    nchunks: usize,
    front: Pos,
    back: Pos,
    marker: PhantomData<&'a mut (K, V)>,
}
impl<'a, K, V> RawCursor<'a, K, V> {
    fn new(chunks: &'a mut Vec<Vec<(K, V)>>, front: Pos, back: Pos) -> RawCursor<'a, K, V> {
        RawCursor { chunks: chunks.as_mut_ptr(), nchunks: chunks.len(), front, back, marker: PhantomData }
    }
    #[inline]
    fn slice(&self) -> &'a mut [Vec<(K, V)>] {
        unsafe { crate::slice::from_raw_parts_mut(self.chunks, self.nchunks) }
    }
    #[inline]
    fn next(&mut self) -> Option<&'a mut (K, V)> {
        if self.front == self.back {
            return None;
        }
        let s = self.slice();
        let p = self.front;
        advance(s, &mut self.front);
        let r: *mut (K, V) = &mut s[p.c][p.i];
        Some(unsafe { &mut *r })
    }
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut (K, V)> {
        if self.front == self.back {
            return None;
        }
        let s = self.slice();
        retreat(s, &mut self.back);
        let p = self.back;
        let r: *mut (K, V) = &mut s[p.c][p.i];
        Some(unsafe { &mut *r })
    }
}

pub struct Iter<'a, K, V> {
    chunks: &'a [Vec<(K, V)>],
    front: Pos,
    back: Pos,
    remaining: usize,
}
impl<'a, K, V> Clone for Iter<'a, K, V> {
    fn clone(&self) -> Iter<'a, K, V> {
        Iter { chunks: self.chunks, front: self.front, back: self.back, remaining: self.remaining }
    }
}
impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        let p = self.front;
        advance(self.chunks, &mut self.front);
        let kv = &self.chunks[p.c][p.i];
        Some((&kv.0, &kv.1))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
    fn count(self) -> usize {
        self.remaining
    }
    fn last(mut self) -> Option<(&'a K, &'a V)> {
        self.next_back()
    }
}
impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        retreat(self.chunks, &mut self.back);
        let p = self.back;
        let kv = &self.chunks[p.c][p.i];
        Some((&kv.0, &kv.1))
    }
}
impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {
    fn len(&self) -> usize {
        self.remaining
    }
}
impl<'a, K, V> FusedIterator for Iter<'a, K, V> {}
impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for Iter<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct IterMut<'a, K, V> {
    raw: RawCursor<'a, K, V>,
    remaining: usize,
}
impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        match self.raw.next() {
            Some(kv) => Some((&kv.0, &mut kv.1)),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}
impl<'a, K, V> DoubleEndedIterator for IterMut<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        match self.raw.next_back() {
            Some(kv) => Some((&kv.0, &mut kv.1)),
            None => None,
        }
    }
}
impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {
    fn len(&self) -> usize {
        self.remaining
    }
}
impl<'a, K, V> FusedIterator for IterMut<'a, K, V> {}

pub struct Keys<'a, K, V> {
    inner: Iter<'a, K, V>,
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
            Some((k, _)) => Some(k),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    fn count(self) -> usize {
        self.inner.remaining
    }
    fn last(mut self) -> Option<&'a K> {
        self.next_back()
    }
}
impl<'a, K, V> DoubleEndedIterator for Keys<'a, K, V> {
    fn next_back(&mut self) -> Option<&'a K> {
        match self.inner.next_back() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
}
impl<'a, K, V> ExactSizeIterator for Keys<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.remaining
    }
}
impl<'a, K, V> FusedIterator for Keys<'a, K, V> {}
impl<'a, K: fmt::Debug, V> fmt::Debug for Keys<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct Values<'a, K, V> {
    inner: Iter<'a, K, V>,
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
            Some((_, v)) => Some(v),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    fn count(self) -> usize {
        self.inner.remaining
    }
    fn last(mut self) -> Option<&'a V> {
        self.next_back()
    }
}
impl<'a, K, V> DoubleEndedIterator for Values<'a, K, V> {
    fn next_back(&mut self) -> Option<&'a V> {
        match self.inner.next_back() {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
}
impl<'a, K, V> ExactSizeIterator for Values<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.remaining
    }
}
impl<'a, K, V> FusedIterator for Values<'a, K, V> {}
impl<'a, K, V: fmt::Debug> fmt::Debug for Values<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct ValuesMut<'a, K, V> {
    inner: IterMut<'a, K, V>,
}
impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
    type Item = &'a mut V;
    #[inline]
    fn next(&mut self) -> Option<&'a mut V> {
        match self.inner.next() {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<'a, K, V> DoubleEndedIterator for ValuesMut<'a, K, V> {
    fn next_back(&mut self) -> Option<&'a mut V> {
        match self.inner.next_back() {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
}
impl<'a, K, V> ExactSizeIterator for ValuesMut<'a, K, V> {
    fn len(&self) -> usize {
        self.inner.remaining
    }
}
impl<'a, K, V> FusedIterator for ValuesMut<'a, K, V> {}

pub struct IntoIter<K, V> {
    inner: crate::vec::IntoIter<(K, V)>,
}
impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<K, V> DoubleEndedIterator for IntoIter<K, V> {
    fn next_back(&mut self) -> Option<(K, V)> {
        self.inner.next_back()
    }
}
impl<K, V> ExactSizeIterator for IntoIter<K, V> {}
impl<K, V> FusedIterator for IntoIter<K, V> {}

pub struct IntoKeys<K, V> {
    inner: IntoIter<K, V>,
}
impl<K, V> Iterator for IntoKeys<K, V> {
    type Item = K;
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
impl<K, V> DoubleEndedIterator for IntoKeys<K, V> {
    fn next_back(&mut self) -> Option<K> {
        match self.inner.next_back() {
            Some((k, _)) => Some(k),
            None => None,
        }
    }
}
impl<K, V> ExactSizeIterator for IntoKeys<K, V> {}

pub struct IntoValues<K, V> {
    inner: IntoIter<K, V>,
}
impl<K, V> Iterator for IntoValues<K, V> {
    type Item = V;
    fn next(&mut self) -> Option<V> {
        match self.inner.next() {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<K, V> DoubleEndedIterator for IntoValues<K, V> {
    fn next_back(&mut self) -> Option<V> {
        match self.inner.next_back() {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
}
impl<K, V> ExactSizeIterator for IntoValues<K, V> {}

pub struct Range<'a, K, V> {
    chunks: &'a [Vec<(K, V)>],
    front: Pos,
    back: Pos,
}
impl<'a, K, V> Clone for Range<'a, K, V> {
    fn clone(&self) -> Range<'a, K, V> {
        Range { chunks: self.chunks, front: self.front, back: self.back }
    }
}
impl<'a, K, V> Iterator for Range<'a, K, V> {
    type Item = (&'a K, &'a V);
    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.front == self.back {
            return None;
        }
        let p = self.front;
        advance(self.chunks, &mut self.front);
        let kv = &self.chunks[p.c][p.i];
        Some((&kv.0, &kv.1))
    }
    fn last(mut self) -> Option<(&'a K, &'a V)> {
        self.next_back()
    }
}
impl<'a, K, V> DoubleEndedIterator for Range<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        if self.front == self.back {
            return None;
        }
        retreat(self.chunks, &mut self.back);
        let p = self.back;
        let kv = &self.chunks[p.c][p.i];
        Some((&kv.0, &kv.1))
    }
}
impl<'a, K, V> FusedIterator for Range<'a, K, V> {}
impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for Range<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

pub struct RangeMut<'a, K, V> {
    raw: RawCursor<'a, K, V>,
}
impl<'a, K, V> Iterator for RangeMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        match self.raw.next() {
            Some(kv) => Some((&kv.0, &mut kv.1)),
            None => None,
        }
    }
}
impl<'a, K, V> DoubleEndedIterator for RangeMut<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
        match self.raw.next_back() {
            Some(kv) => Some((&kv.0, &mut kv.1)),
            None => None,
        }
    }
}
impl<'a, K, V> FusedIterator for RangeMut<'a, K, V> {}

// ---- entry API ---------------------------------------------------------------

pub enum Entry<'a, K, V> {
    Vacant(VacantEntry<'a, K, V>),
    Occupied(OccupiedEntry<'a, K, V>),
}

pub struct VacantEntry<'a, K, V> {
    map: &'a mut BTreeMap<K, V>,
    key: K,
    pos: Pos,
}

pub struct OccupiedEntry<'a, K, V> {
    map: &'a mut BTreeMap<K, V>,
    pos: Pos,
}

impl<'a, K: Ord, V> Entry<'a, K, V> {
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
impl<'a, K: Ord, V: Default> Entry<'a, K, V> {
    pub fn or_default(self) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(V::default()),
        }
    }
}

impl<'a, K: Ord, V> VacantEntry<'a, K, V> {
    pub fn key(&self) -> &K {
        &self.key
    }
    pub fn into_key(self) -> K {
        self.key
    }
    pub fn insert(self, value: V) -> &'a mut V {
        let p = self.map.insert_at(self.pos, self.key, value);
        &mut self.map.at_mut(p).1
    }
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
        let p = self.map.insert_at(self.pos, self.key, value);
        OccupiedEntry { map: self.map, pos: p }
    }
}

impl<'a, K: Ord, V> OccupiedEntry<'a, K, V> {
    pub fn key(&self) -> &K {
        &self.map.at(self.pos).0
    }
    pub fn get(&self) -> &V {
        &self.map.at(self.pos).1
    }
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.map.at_mut(self.pos).1
    }
    pub fn into_mut(self) -> &'a mut V {
        &mut self.map.at_mut(self.pos).1
    }
    pub fn insert(&mut self, value: V) -> V {
        mem::replace(&mut self.map.at_mut(self.pos).1, value)
    }
    pub fn remove(self) -> V {
        self.map.remove_at(self.pos).1
    }
    pub fn remove_entry(self) -> (K, V) {
        self.map.remove_at(self.pos)
    }
}

impl<'a, K: Ord + fmt::Debug, V: fmt::Debug> fmt::Debug for Entry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Entry::Vacant(e) => f.debug_tuple("Entry").field(e).finish(),
            Entry::Occupied(e) => f.debug_tuple("Entry").field(e).finish(),
        }
    }
}
impl<'a, K: Ord + fmt::Debug, V> fmt::Debug for VacantEntry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry").field(self.key()).finish()
    }
}
impl<'a, K: Ord + fmt::Debug, V: fmt::Debug> fmt::Debug for OccupiedEntry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OccupiedEntry").field("key", self.key()).field("value", self.get()).finish()
    }
}
