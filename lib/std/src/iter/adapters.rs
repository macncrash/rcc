use crate::iter::traits::{DoubleEndedIterator, ExactSizeIterator, IntoIterator, Iterator};

pub struct Map<I, F> {
    iter: I,
    f: F,
}
impl<I, F> Map<I, F> {
    pub(crate) fn new(iter: I, f: F) -> Map<I, F> {
        Map { iter, f }
    }
}
impl<B, I: Iterator, F: FnMut(I::Item) -> B> Iterator for Map<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> {
        match self.iter.next() {
            Some(x) => Some((self.f)(x)),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<B, I: DoubleEndedIterator, F: FnMut(I::Item) -> B> DoubleEndedIterator for Map<I, F> {
    fn next_back(&mut self) -> Option<B> {
        match self.iter.next_back() {
            Some(x) => Some((self.f)(x)),
            None => None,
        }
    }
}
impl<B, I: ExactSizeIterator, F: FnMut(I::Item) -> B> ExactSizeIterator for Map<I, F> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
impl<I: Clone, F: Clone> Clone for Map<I, F> {
    fn clone(&self) -> Map<I, F> {
        Map { iter: self.iter.clone(), f: self.f.clone() }
    }
}

#[derive(Clone)]
pub struct Filter<I, P> {
    iter: I,
    predicate: P,
}
impl<I, P> Filter<I, P> {
    pub(crate) fn new(iter: I, predicate: P) -> Filter<I, P> {
        Filter { iter, predicate }
    }
}
impl<I: Iterator, P: FnMut(&I::Item) -> bool> Iterator for Filter<I, P> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        while let Some(x) = self.iter.next() {
            if (self.predicate)(&x) {
                return Some(x);
            }
        }
        None
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.iter.size_hint().1)
    }
}
impl<I: DoubleEndedIterator, P: FnMut(&I::Item) -> bool> DoubleEndedIterator for Filter<I, P> {
    fn next_back(&mut self) -> Option<I::Item> {
        while let Some(x) = self.iter.next_back() {
            if (self.predicate)(&x) {
                return Some(x);
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct FilterMap<I, F> {
    iter: I,
    f: F,
}
impl<I, F> FilterMap<I, F> {
    pub(crate) fn new(iter: I, f: F) -> FilterMap<I, F> {
        FilterMap { iter, f }
    }
}
impl<B, I: Iterator, F: FnMut(I::Item) -> Option<B>> Iterator for FilterMap<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> {
        while let Some(x) = self.iter.next() {
            if let Some(y) = (self.f)(x) {
                return Some(y);
            }
        }
        None
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.iter.size_hint().1)
    }
}
impl<B, I: DoubleEndedIterator, F: FnMut(I::Item) -> Option<B>> DoubleEndedIterator for FilterMap<I, F> {
    fn next_back(&mut self) -> Option<B> {
        while let Some(x) = self.iter.next_back() {
            if let Some(y) = (self.f)(x) {
                return Some(y);
            }
        }
        None
    }
}

pub struct Enumerate<I> {
    iter: I,
    count: usize,
}
impl<I> Enumerate<I> {
    pub(crate) fn new(iter: I) -> Enumerate<I> {
        Enumerate { iter, count: 0 }
    }
}
impl<I: Iterator> Iterator for Enumerate<I> {
    type Item = (usize, I::Item);
    fn next(&mut self) -> Option<(usize, I::Item)> {
        match self.iter.next() {
            Some(x) => {
                let i = self.count;
                self.count += 1;
                Some((i, x))
            }
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<I: ExactSizeIterator + DoubleEndedIterator> DoubleEndedIterator for Enumerate<I> {
    fn next_back(&mut self) -> Option<(usize, I::Item)> {
        match self.iter.next_back() {
            Some(x) => {
                let len = self.iter.len();
                Some((self.count + len, x))
            }
            None => None,
        }
    }
}
impl<I: ExactSizeIterator> ExactSizeIterator for Enumerate<I> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
impl<I: Clone> Clone for Enumerate<I> {
    fn clone(&self) -> Enumerate<I> {
        Enumerate { iter: self.iter.clone(), count: self.count }
    }
}

pub struct Rev<I> {
    iter: I,
}
impl<I> Rev<I> {
    pub(crate) fn new(iter: I) -> Rev<I> {
        Rev { iter }
    }
}
impl<I: DoubleEndedIterator> Iterator for Rev<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        self.iter.next_back()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<I: DoubleEndedIterator> DoubleEndedIterator for Rev<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        self.iter.next()
    }
}
impl<I: ExactSizeIterator + DoubleEndedIterator> ExactSizeIterator for Rev<I> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
impl<I: Clone> Clone for Rev<I> {
    fn clone(&self) -> Rev<I> {
        Rev { iter: self.iter.clone() }
    }
}

#[derive(Clone)]
pub struct Chain<A, B> {
    a: Option<A>,
    b: Option<B>,
}
impl<A, B> Chain<A, B> {
    pub(crate) fn new(a: A, b: B) -> Chain<A, B> {
        Chain { a: Some(a), b: Some(b) }
    }
}
impl<A: Iterator, B: Iterator<Item = A::Item>> Iterator for Chain<A, B> {
    type Item = A::Item;
    fn next(&mut self) -> Option<A::Item> {
        if let Some(a) = self.a.as_mut() {
            match a.next() {
                Some(x) => return Some(x),
                None => self.a = None,
            }
        }
        match self.b.as_mut() {
            Some(b) => b.next(),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (al, ah) = match self.a.as_ref() {
            Some(a) => a.size_hint(),
            None => (0, Some(0)),
        };
        let (bl, bh) = match self.b.as_ref() {
            Some(b) => b.size_hint(),
            None => (0, Some(0)),
        };
        let hi = match (ah, bh) {
            (Some(x), Some(y)) => Some(x + y),
            _ => None,
        };
        (al + bl, hi)
    }
}
impl<A: DoubleEndedIterator, B: DoubleEndedIterator<Item = A::Item>> DoubleEndedIterator for Chain<A, B> {
    fn next_back(&mut self) -> Option<A::Item> {
        if let Some(b) = self.b.as_mut() {
            match b.next_back() {
                Some(x) => return Some(x),
                None => self.b = None,
            }
        }
        match self.a.as_mut() {
            Some(a) => a.next_back(),
            None => None,
        }
    }
}

#[derive(Clone)]
pub struct Zip<A, B> {
    a: A,
    b: B,
}
impl<A, B> Zip<A, B> {
    pub(crate) fn new(a: A, b: B) -> Zip<A, B> {
        Zip { a, b }
    }
}
impl<A: Iterator, B: Iterator> Iterator for Zip<A, B> {
    type Item = (A::Item, B::Item);
    fn next(&mut self) -> Option<(A::Item, B::Item)> {
        let x = match self.a.next() {
            Some(x) => x,
            None => return None,
        };
        match self.b.next() {
            Some(y) => Some((x, y)),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (al, ah) = self.a.size_hint();
        let (bl, bh) = self.b.size_hint();
        let lo = if al < bl { al } else { bl };
        let hi = match (ah, bh) {
            (Some(x), Some(y)) => Some(if x < y { x } else { y }),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        };
        (lo, hi)
    }
}
impl<A: ExactSizeIterator, B: ExactSizeIterator> ExactSizeIterator for Zip<A, B> {
    fn len(&self) -> usize {
        let a = self.a.len();
        let b = self.b.len();
        if a < b { a } else { b }
    }
}
impl<A: ExactSizeIterator + DoubleEndedIterator, B: ExactSizeIterator + DoubleEndedIterator> DoubleEndedIterator for Zip<A, B> {
    fn next_back(&mut self) -> Option<(A::Item, B::Item)> {
        let al = self.a.len();
        let bl = self.b.len();
        let mut i = al;
        while i > bl {
            self.a.next_back();
            i -= 1;
        }
        let mut j = bl;
        while j > al {
            self.b.next_back();
            j -= 1;
        }
        match (self.a.next_back(), self.b.next_back()) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        }
    }
}

pub struct Peekable<I: Iterator> {
    iter: I,
    peeked: Option<Option<I::Item>>,
}
impl<I: Iterator> Peekable<I> {
    pub(crate) fn new(iter: I) -> Peekable<I> {
        Peekable { iter, peeked: None }
    }
    pub fn peek(&mut self) -> Option<&I::Item> {
        if self.peeked.is_none() {
            self.peeked = Some(self.iter.next());
        }
        match self.peeked.as_ref() {
            Some(p) => p.as_ref(),
            None => None,
        }
    }
    pub fn peek_mut(&mut self) -> Option<&mut I::Item> {
        if self.peeked.is_none() {
            self.peeked = Some(self.iter.next());
        }
        match self.peeked.as_mut() {
            Some(p) => p.as_mut(),
            None => None,
        }
    }
    pub fn next_if<F: FnOnce(&I::Item) -> bool>(&mut self, func: F) -> Option<I::Item> {
        match self.next() {
            Some(x) => {
                if func(&x) {
                    Some(x)
                } else {
                    self.peeked = Some(Some(x));
                    None
                }
            }
            None => {
                self.peeked = Some(None);
                None
            }
        }
    }
    pub fn next_if_eq<T: ?Sized>(&mut self, expected: &T) -> Option<I::Item>
    where
        I::Item: PartialEq<T>,
    {
        self.next_if(|x| *x == *expected)
    }
}
impl<I: Iterator> Iterator for Peekable<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        match self.peeked.take() {
            Some(v) => v,
            None => self.iter.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let extra = match self.peeked.as_ref() {
            Some(Some(_)) => 1,
            Some(None) => return (0, Some(0)),
            None => 0,
        };
        let (lo, hi) = self.iter.size_hint();
        (lo + extra, match hi {
            Some(h) => Some(h + extra),
            None => None,
        })
    }
}

#[derive(Clone)]
pub struct Skip<I> {
    iter: I,
    n: usize,
}
impl<I> Skip<I> {
    pub(crate) fn new(iter: I, n: usize) -> Skip<I> {
        Skip { iter, n }
    }
}
impl<I: Iterator> Iterator for Skip<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        while self.n > 0 {
            self.n -= 1;
            if let None = self.iter.next() {
                return None;
            }
        }
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lo, hi) = self.iter.size_hint();
        (if lo > self.n { lo - self.n } else { 0 }, match hi {
            Some(h) => Some(if h > self.n { h - self.n } else { 0 }),
            None => None,
        })
    }
}
impl<I: ExactSizeIterator> ExactSizeIterator for Skip<I> {
    fn len(&self) -> usize {
        let l = self.iter.len();
        if l > self.n { l - self.n } else { 0 }
    }
}
impl<I: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for Skip<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        if self.len() > 0 { self.iter.next_back() } else { None }
    }
}

#[derive(Clone)]
pub struct Take<I> {
    iter: I,
    n: usize,
}
impl<I> Take<I> {
    pub(crate) fn new(iter: I, n: usize) -> Take<I> {
        Take { iter, n }
    }
}
impl<I: Iterator> Iterator for Take<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lo, hi) = self.iter.size_hint();
        let lo = if lo < self.n { lo } else { self.n };
        let hi = match hi {
            Some(h) => if h < self.n { h } else { self.n },
            None => self.n,
        };
        (lo, Some(hi))
    }
}
impl<I: ExactSizeIterator> ExactSizeIterator for Take<I> {
    fn len(&self) -> usize {
        let l = self.iter.len();
        if l < self.n { l } else { self.n }
    }
}
impl<I: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for Take<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        if self.n == 0 {
            return None;
        }
        let len = self.iter.len();
        let mut extra = if len > self.n { len - self.n } else { 0 };
        while extra > 0 {
            self.iter.next_back();
            extra -= 1;
        }
        self.n -= 1;
        self.iter.next_back()
    }
}

#[derive(Clone)]
pub struct SkipWhile<I, P> {
    iter: I,
    done: bool,
    predicate: P,
}
impl<I, P> SkipWhile<I, P> {
    pub(crate) fn new(iter: I, predicate: P) -> SkipWhile<I, P> {
        SkipWhile { iter, done: false, predicate }
    }
}
impl<I: Iterator, P: FnMut(&I::Item) -> bool> Iterator for SkipWhile<I, P> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        while let Some(x) = self.iter.next() {
            if self.done || !(self.predicate)(&x) {
                self.done = true;
                return Some(x);
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct TakeWhile<I, P> {
    iter: I,
    done: bool,
    predicate: P,
}
impl<I, P> TakeWhile<I, P> {
    pub(crate) fn new(iter: I, predicate: P) -> TakeWhile<I, P> {
        TakeWhile { iter, done: false, predicate }
    }
}
impl<I: Iterator, P: FnMut(&I::Item) -> bool> Iterator for TakeWhile<I, P> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        if self.done {
            return None;
        }
        match self.iter.next() {
            Some(x) => {
                if (self.predicate)(&x) {
                    Some(x)
                } else {
                    self.done = true;
                    None
                }
            }
            None => None,
        }
    }
}

pub struct MapWhile<I, P> {
    iter: I,
    predicate: P,
}
impl<I, P> MapWhile<I, P> {
    pub(crate) fn new(iter: I, predicate: P) -> MapWhile<I, P> {
        MapWhile { iter, predicate }
    }
}
impl<B, I: Iterator, P: FnMut(I::Item) -> Option<B>> Iterator for MapWhile<I, P> {
    type Item = B;
    fn next(&mut self) -> Option<B> {
        match self.iter.next() {
            Some(x) => (self.predicate)(x),
            None => None,
        }
    }
}

pub struct Scan<I, St, F> {
    iter: I,
    state: St,
    f: F,
}
impl<I, St, F> Scan<I, St, F> {
    pub(crate) fn new(iter: I, state: St, f: F) -> Scan<I, St, F> {
        Scan { iter, state, f }
    }
}
impl<B, I: Iterator, St, F: FnMut(&mut St, I::Item) -> Option<B>> Iterator for Scan<I, St, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> {
        match self.iter.next() {
            Some(x) => (self.f)(&mut self.state, x),
            None => None,
        }
    }
}

pub struct Inspect<I, F> {
    iter: I,
    f: F,
}
impl<I, F> Inspect<I, F> {
    pub(crate) fn new(iter: I, f: F) -> Inspect<I, F> {
        Inspect { iter, f }
    }
}
impl<I: Iterator, F: FnMut(&I::Item)> Iterator for Inspect<I, F> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        match self.iter.next() {
            Some(x) => {
                (self.f)(&x);
                Some(x)
            }
            None => None,
        }
    }
}

#[derive(Clone)]
pub struct Fuse<I> {
    iter: Option<I>,
}
impl<I> Fuse<I> {
    pub(crate) fn new(iter: I) -> Fuse<I> {
        Fuse { iter: Some(iter) }
    }
}
impl<I: Iterator> Iterator for Fuse<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        let r = match self.iter.as_mut() {
            Some(it) => it.next(),
            None => return None,
        };
        if r.is_none() {
            self.iter = None;
        }
        r
    }
}

pub struct FlatMap<I, U: IntoIterator, F> {
    iter: I,
    f: F,
    front: Option<U::IntoIter>,
    back: Option<U::IntoIter>,
}
impl<I, U: IntoIterator, F> FlatMap<I, U, F> {
    pub(crate) fn new(iter: I, f: F) -> FlatMap<I, U, F> {
        FlatMap { iter, f, front: None, back: None }
    }
}
impl<I: Iterator, U: IntoIterator, F: FnMut(I::Item) -> U> Iterator for FlatMap<I, U, F> {
    type Item = U::Item;
    fn next(&mut self) -> Option<U::Item> {
        loop {
            if let Some(inner) = self.front.as_mut() {
                match inner.next() {
                    Some(x) => return Some(x),
                    None => self.front = None,
                }
            }
            match self.iter.next() {
                Some(x) => self.front = Some((self.f)(x).into_iter()),
                None => {
                    return match self.back.as_mut() {
                        Some(inner) => inner.next(),
                        None => None,
                    };
                }
            }
        }
    }
}
impl<I: DoubleEndedIterator, U: IntoIterator, F: FnMut(I::Item) -> U> DoubleEndedIterator for FlatMap<I, U, F>
where
    U::IntoIter: DoubleEndedIterator,
{
    fn next_back(&mut self) -> Option<U::Item> {
        loop {
            if let Some(inner) = self.back.as_mut() {
                match inner.next_back() {
                    Some(x) => return Some(x),
                    None => self.back = None,
                }
            }
            match self.iter.next_back() {
                Some(x) => self.back = Some((self.f)(x).into_iter()),
                None => {
                    return match self.front.as_mut() {
                        Some(inner) => inner.next_back(),
                        None => None,
                    };
                }
            }
        }
    }
}

pub struct Flatten<I: Iterator>
where
    I::Item: IntoIterator,
{
    iter: I,
    front: Option<<I::Item as IntoIterator>::IntoIter>,
    back: Option<<I::Item as IntoIterator>::IntoIter>,
}
impl<I: Iterator> Flatten<I>
where
    I::Item: IntoIterator,
{
    pub(crate) fn new(iter: I) -> Flatten<I> {
        Flatten { iter, front: None, back: None }
    }
}
impl<I: Iterator> Iterator for Flatten<I>
where
    I::Item: IntoIterator,
{
    type Item = <I::Item as IntoIterator>::Item;
    fn next(&mut self) -> Option<<I::Item as IntoIterator>::Item> {
        loop {
            if let Some(inner) = self.front.as_mut() {
                match inner.next() {
                    Some(x) => return Some(x),
                    None => self.front = None,
                }
            }
            match self.iter.next() {
                Some(x) => self.front = Some(x.into_iter()),
                None => {
                    return match self.back.as_mut() {
                        Some(inner) => inner.next(),
                        None => None,
                    };
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct Cloned<I> {
    iter: I,
}
impl<I> Cloned<I> {
    pub(crate) fn new(iter: I) -> Cloned<I> {
        Cloned { iter }
    }
}
impl<'a, T: 'a + Clone, I: Iterator<Item = &'a T>> Iterator for Cloned<I> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self.iter.next() {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<'a, T: 'a + Clone, I: DoubleEndedIterator<Item = &'a T>> DoubleEndedIterator for Cloned<I> {
    fn next_back(&mut self) -> Option<T> {
        match self.iter.next_back() {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
}
impl<'a, T: 'a + Clone, I: ExactSizeIterator<Item = &'a T>> ExactSizeIterator for Cloned<I> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

#[derive(Clone)]
pub struct Copied<I> {
    iter: I,
}
impl<I> Copied<I> {
    pub(crate) fn new(iter: I) -> Copied<I> {
        Copied { iter }
    }
}
impl<'a, T: 'a + Copy, I: Iterator<Item = &'a T>> Iterator for Copied<I> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self.iter.next() {
            Some(x) => Some(*x),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<'a, T: 'a + Copy, I: DoubleEndedIterator<Item = &'a T>> DoubleEndedIterator for Copied<I> {
    fn next_back(&mut self) -> Option<T> {
        match self.iter.next_back() {
            Some(x) => Some(*x),
            None => None,
        }
    }
}
impl<'a, T: 'a + Copy, I: ExactSizeIterator<Item = &'a T>> ExactSizeIterator for Copied<I> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

pub struct Cycle<I> {
    orig: I,
    iter: I,
}
impl<I: Clone> Cycle<I> {
    pub(crate) fn new(iter: I) -> Cycle<I> {
        Cycle { orig: iter.clone(), iter }
    }
}
impl<I: Clone + Iterator> Iterator for Cycle<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        match self.iter.next() {
            Some(x) => Some(x),
            None => {
                self.iter = self.orig.clone();
                self.iter.next()
            }
        }
    }
}

#[derive(Clone)]
pub struct StepBy<I> {
    iter: I,
    step: usize,
    first: bool,
}
impl<I> StepBy<I> {
    pub(crate) fn new(iter: I, step: usize) -> StepBy<I> {
        if step == 0 {
            panic!("assertion failed: step != 0");
        }
        StepBy { iter, step: step - 1, first: true }
    }
}
impl<I: Iterator> Iterator for StepBy<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        if self.first {
            self.first = false;
            self.iter.next()
        } else {
            self.iter.nth(self.step)
        }
    }
}
