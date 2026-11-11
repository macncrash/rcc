use crate::cmp::Ordering;
use crate::iter::adapters::*;

#[lang = "Iterator"]
pub trait Iterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
    fn count(mut self) -> usize
    where
        Self: Sized,
    {
        let mut n = 0;
        while let Some(_) = self.next() {
            n += 1;
        }
        n
    }
    fn last(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let mut last = None;
        while let Some(x) = self.next() {
            last = Some(x);
        }
        last
    }
    fn nth(&mut self, mut n: usize) -> Option<Self::Item> {
        while n > 0 {
            if let None = self.next() {
                return None;
            }
            n -= 1;
        }
        self.next()
    }
    fn step_by(self, step: usize) -> StepBy<Self>
    where
        Self: Sized,
    {
        StepBy::new(self, step)
    }
    fn chain<U: IntoIterator<Item = Self::Item>>(self, other: U) -> Chain<Self, U::IntoIter>
    where
        Self: Sized,
    {
        Chain::new(self, other.into_iter())
    }
    fn zip<U: IntoIterator>(self, other: U) -> Zip<Self, U::IntoIter>
    where
        Self: Sized,
    {
        Zip::new(self, other.into_iter())
    }
    fn map<B, F: FnMut(Self::Item) -> B>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
    {
        Map::new(self, f)
    }
    fn for_each<F: FnMut(Self::Item)>(mut self, mut f: F)
    where
        Self: Sized,
    {
        while let Some(x) = self.next() {
            f(x);
        }
    }
    fn filter<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> Filter<Self, P>
    where
        Self: Sized,
    {
        Filter::new(self, predicate)
    }
    fn filter_map<B, F: FnMut(Self::Item) -> Option<B>>(self, f: F) -> FilterMap<Self, F>
    where
        Self: Sized,
    {
        FilterMap::new(self, f)
    }
    fn enumerate(self) -> Enumerate<Self>
    where
        Self: Sized,
    {
        Enumerate::new(self)
    }
    fn peekable(self) -> Peekable<Self>
    where
        Self: Sized,
    {
        Peekable::new(self)
    }
    fn skip_while<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> SkipWhile<Self, P>
    where
        Self: Sized,
    {
        SkipWhile::new(self, predicate)
    }
    fn take_while<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> TakeWhile<Self, P>
    where
        Self: Sized,
    {
        TakeWhile::new(self, predicate)
    }
    fn map_while<B, P: FnMut(Self::Item) -> Option<B>>(self, predicate: P) -> MapWhile<Self, P>
    where
        Self: Sized,
    {
        MapWhile::new(self, predicate)
    }
    fn skip(self, n: usize) -> Skip<Self>
    where
        Self: Sized,
    {
        Skip::new(self, n)
    }
    fn take(self, n: usize) -> Take<Self>
    where
        Self: Sized,
    {
        Take::new(self, n)
    }
    fn scan<St, B, F: FnMut(&mut St, Self::Item) -> Option<B>>(self, initial_state: St, f: F) -> Scan<Self, St, F>
    where
        Self: Sized,
    {
        Scan::new(self, initial_state, f)
    }
    fn flat_map<U: IntoIterator, F: FnMut(Self::Item) -> U>(self, f: F) -> FlatMap<Self, U, F>
    where
        Self: Sized,
    {
        FlatMap::new(self, f)
    }
    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        Flatten::new(self)
    }
    fn fuse(self) -> Fuse<Self>
    where
        Self: Sized,
    {
        Fuse::new(self)
    }
    fn inspect<F: FnMut(&Self::Item)>(self, f: F) -> Inspect<Self, F>
    where
        Self: Sized,
    {
        Inspect::new(self, f)
    }
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
    fn collect<B: FromIterator<Self::Item>>(self) -> B
    where
        Self: Sized,
    {
        FromIterator::from_iter(self)
    }
    fn partition<B, F>(mut self, mut f: F) -> (B, B)
    where
        Self: Sized,
        B: Default + Extend<Self::Item>,
        F: FnMut(&Self::Item) -> bool,
    {
        let mut left: B = Default::default();
        let mut right: B = Default::default();
        while let Some(x) = self.next() {
            if f(&x) {
                left.extend(Some(x));
            } else {
                right.extend(Some(x));
            }
        }
        (left, right)
    }
    fn try_fold<B, F, R>(&mut self, init: B, mut f: F) -> R
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> R,
        R: crate::ops::Try<Output = B>,
    {
        let mut acc = init;
        while let Some(x) = self.next() {
            match f(acc, x).branch() {
                crate::ops::ControlFlow::Continue(v) => acc = v,
                crate::ops::ControlFlow::Break(r) => return R::from_residual(r),
            }
        }
        R::from_output(acc)
    }
    fn try_for_each<F, R>(&mut self, mut f: F) -> R
    where
        Self: Sized,
        F: FnMut(Self::Item) -> R,
        R: crate::ops::Try<Output = ()>,
    {
        while let Some(x) = self.next() {
            match f(x).branch() {
                crate::ops::ControlFlow::Continue(_) => {}
                crate::ops::ControlFlow::Break(r) => return R::from_residual(r),
            }
        }
        R::from_output(())
    }
    fn fold<B, F: FnMut(B, Self::Item) -> B>(mut self, init: B, mut f: F) -> B
    where
        Self: Sized,
    {
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
    fn reduce<F: FnMut(Self::Item, Self::Item) -> Self::Item>(mut self, mut f: F) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let first = match self.next() {
            Some(x) => x,
            None => return None,
        };
        let mut acc = first;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        Some(acc)
    }
    fn all<F: FnMut(Self::Item) -> bool>(&mut self, mut f: F) -> bool
    where
        Self: Sized,
    {
        while let Some(x) = self.next() {
            if !f(x) {
                return false;
            }
        }
        true
    }
    fn any<F: FnMut(Self::Item) -> bool>(&mut self, mut f: F) -> bool
    where
        Self: Sized,
    {
        while let Some(x) = self.next() {
            if f(x) {
                return true;
            }
        }
        false
    }
    fn find<P: FnMut(&Self::Item) -> bool>(&mut self, mut predicate: P) -> Option<Self::Item>
    where
        Self: Sized,
    {
        while let Some(x) = self.next() {
            if predicate(&x) {
                return Some(x);
            }
        }
        None
    }
    fn find_map<B, F: FnMut(Self::Item) -> Option<B>>(&mut self, mut f: F) -> Option<B>
    where
        Self: Sized,
    {
        while let Some(x) = self.next() {
            if let Some(b) = f(x) {
                return Some(b);
            }
        }
        None
    }
    fn position<P: FnMut(Self::Item) -> bool>(&mut self, mut predicate: P) -> Option<usize>
    where
        Self: Sized,
    {
        let mut i = 0;
        while let Some(x) = self.next() {
            if predicate(x) {
                return Some(i);
            }
            i += 1;
        }
        None
    }
    fn rposition<P: FnMut(Self::Item) -> bool>(&mut self, mut predicate: P) -> Option<usize>
    where
        Self: Sized + ExactSizeIterator + DoubleEndedIterator,
    {
        let mut i = self.len();
        while let Some(x) = self.next_back() {
            i -= 1;
            if predicate(x) {
                return Some(i);
            }
        }
        None
    }
    fn max(mut self) -> Option<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut best = match self.next() {
            Some(x) => x,
            None => return None,
        };
        while let Some(x) = self.next() {
            if x >= best {
                best = x;
            }
        }
        Some(best)
    }
    fn min(mut self) -> Option<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut best = match self.next() {
            Some(x) => x,
            None => return None,
        };
        while let Some(x) = self.next() {
            if x < best {
                best = x;
            }
        }
        Some(best)
    }
    fn max_by_key<B: Ord, F: FnMut(&Self::Item) -> B>(mut self, mut f: F) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let mut best = match self.next() {
            Some(x) => x,
            None => return None,
        };
        let mut best_key = f(&best);
        while let Some(x) = self.next() {
            let k = f(&x);
            if k >= best_key {
                best = x;
                best_key = k;
            }
        }
        Some(best)
    }
    fn min_by_key<B: Ord, F: FnMut(&Self::Item) -> B>(mut self, mut f: F) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let mut best = match self.next() {
            Some(x) => x,
            None => return None,
        };
        let mut best_key = f(&best);
        while let Some(x) = self.next() {
            let k = f(&x);
            if k < best_key {
                best = x;
                best_key = k;
            }
        }
        Some(best)
    }
    fn max_by<F: FnMut(&Self::Item, &Self::Item) -> Ordering>(mut self, mut compare: F) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let mut best = match self.next() {
            Some(x) => x,
            None => return None,
        };
        while let Some(x) = self.next() {
            if let Ordering::Greater = compare(&best, &x) {
            } else {
                best = x;
            }
        }
        Some(best)
    }
    fn min_by<F: FnMut(&Self::Item, &Self::Item) -> Ordering>(mut self, mut compare: F) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let mut best = match self.next() {
            Some(x) => x,
            None => return None,
        };
        while let Some(x) = self.next() {
            if let Ordering::Greater = compare(&best, &x) {
                best = x;
            }
        }
        Some(best)
    }
    fn rev(self) -> Rev<Self>
    where
        Self: Sized + DoubleEndedIterator,
    {
        Rev::new(self)
    }
    fn unzip<A, B, FromA, FromB>(mut self) -> (FromA, FromB)
    where
        FromA: Default + Extend<A>,
        FromB: Default + Extend<B>,
        Self: Sized + Iterator<Item = (A, B)>,
    {
        let mut a: FromA = Default::default();
        let mut b: FromB = Default::default();
        while let Some((x, y)) = self.next() {
            a.extend(Some(x));
            b.extend(Some(y));
        }
        (a, b)
    }
    fn copied<'a, T: 'a + Copy>(self) -> Copied<Self>
    where
        Self: Sized + Iterator<Item = &'a T>,
    {
        Copied::new(self)
    }
    fn cloned<'a, T: 'a + Clone>(self) -> Cloned<Self>
    where
        Self: Sized + Iterator<Item = &'a T>,
    {
        Cloned::new(self)
    }
    fn cycle(self) -> Cycle<Self>
    where
        Self: Sized + Clone,
    {
        Cycle::new(self)
    }
    fn sum<S: Sum<Self::Item>>(self) -> S
    where
        Self: Sized,
    {
        Sum::sum(self)
    }
    fn product<P: Product<Self::Item>>(self) -> P
    where
        Self: Sized,
    {
        Product::product(self)
    }
    fn cmp<I>(mut self, other: I) -> Ordering
    where
        I: IntoIterator<Item = Self::Item>,
        Self::Item: Ord,
        Self: Sized,
    {
        let mut other = other.into_iter();
        loop {
            match (self.next(), other.next()) {
                (None, None) => return Ordering::Equal,
                (None, Some(_)) => return Ordering::Less,
                (Some(_), None) => return Ordering::Greater,
                (Some(a), Some(b)) => match a.cmp(&b) {
                    Ordering::Equal => {}
                    r => return r,
                },
            }
        }
    }
    fn eq<I>(mut self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: PartialEq<I::Item>,
        Self: Sized,
    {
        let mut other = other.into_iter();
        loop {
            match (self.next(), other.next()) {
                (None, None) => return true,
                (Some(a), Some(b)) => {
                    if a != b {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }
    fn ne<I>(self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: PartialEq<I::Item>,
        Self: Sized,
    {
        !self.eq(other)
    }
    fn partial_cmp<I>(mut self, other: I) -> Option<Ordering>
    where
        I: IntoIterator,
        Self::Item: PartialOrd<I::Item>,
        Self: Sized,
    {
        let mut other = other.into_iter();
        loop {
            match (self.next(), other.next()) {
                (None, None) => return Some(Ordering::Equal),
                (None, Some(_)) => return Some(Ordering::Less),
                (Some(_), None) => return Some(Ordering::Greater),
                (Some(a), Some(b)) => match a.partial_cmp(&b) {
                    Some(Ordering::Equal) => {}
                    r => return r,
                },
            }
        }
    }
    fn lt<I>(self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: PartialOrd<I::Item>,
        Self: Sized,
    {
        matches!(self.partial_cmp(other), Some(Ordering::Less))
    }
    fn le<I>(self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: PartialOrd<I::Item>,
        Self: Sized,
    {
        matches!(self.partial_cmp(other), Some(Ordering::Less | Ordering::Equal))
    }
    fn gt<I>(self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: PartialOrd<I::Item>,
        Self: Sized,
    {
        matches!(self.partial_cmp(other), Some(Ordering::Greater))
    }
    fn ge<I>(self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: PartialOrd<I::Item>,
        Self: Sized,
    {
        matches!(self.partial_cmp(other), Some(Ordering::Greater | Ordering::Equal))
    }
    fn is_sorted(mut self) -> bool
    where
        Self: Sized,
        Self::Item: PartialOrd,
    {
        let mut prev = match self.next() {
            Some(x) => x,
            None => return true,
        };
        while let Some(x) = self.next() {
            if x < prev {
                return false;
            }
            prev = x;
        }
        true
    }
}

impl<I: Iterator + ?Sized> Iterator for &mut I {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        (**self).next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (**self).size_hint()
    }
}

pub trait DoubleEndedIterator: Iterator {
    fn next_back(&mut self) -> Option<Self::Item>;
    fn nth_back(&mut self, mut n: usize) -> Option<Self::Item> {
        while n > 0 {
            if let None = self.next_back() {
                return None;
            }
            n -= 1;
        }
        self.next_back()
    }
    fn rfind<P: FnMut(&Self::Item) -> bool>(&mut self, mut predicate: P) -> Option<Self::Item>
    where
        Self: Sized,
    {
        while let Some(x) = self.next_back() {
            if predicate(&x) {
                return Some(x);
            }
        }
        None
    }
    fn rfold<B, F: FnMut(B, Self::Item) -> B>(mut self, init: B, mut f: F) -> B
    where
        Self: Sized,
    {
        let mut acc = init;
        while let Some(x) = self.next_back() {
            acc = f(acc, x);
        }
        acc
    }
}

impl<I: DoubleEndedIterator + ?Sized> DoubleEndedIterator for &mut I {
    fn next_back(&mut self) -> Option<I::Item> {
        (**self).next_back()
    }
}

pub trait ExactSizeIterator: Iterator {
    fn len(&self) -> usize {
        self.size_hint().0
    }
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<I: ExactSizeIterator + ?Sized> ExactSizeIterator for &mut I {
    fn len(&self) -> usize {
        (**self).len()
    }
}

pub trait FusedIterator: Iterator {}

#[lang = "IntoIterator"]
pub trait IntoIterator {
    type Item;
    type IntoIter: Iterator<Item = Self::Item>;
    fn into_iter(self) -> Self::IntoIter;
}

impl<I: Iterator> IntoIterator for I {
    type Item = I::Item;
    type IntoIter = I;
    fn into_iter(self) -> I {
        self
    }
}

pub trait FromIterator<A>: Sized {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self;
}

pub trait Extend<A> {
    fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T);
}

impl Extend<()> for () {
    fn extend<T: IntoIterator<Item = ()>>(&mut self, iter: T) {
        let mut it = iter.into_iter();
        while let Some(_) = it.next() {}
    }
}

impl FromIterator<()> for () {
    fn from_iter<T: IntoIterator<Item = ()>>(iter: T) {
        let mut it = iter.into_iter();
        while let Some(_) = it.next() {}
    }
}

/// `Option<V>` collects `Option<A>` items, stopping at the first `None`.
impl<A, V: FromIterator<A>> FromIterator<Option<A>> for Option<V> {
    fn from_iter<T: IntoIterator<Item = Option<A>>>(iter: T) -> Option<V> {
        let mut failed = false;
        let v: V = FromIterator::from_iter(OptionShunt { iter: iter.into_iter(), failed: &mut failed });
        if failed { None } else { Some(v) }
    }
}

struct OptionShunt<'a, I> {
    iter: I,
    failed: &'a mut bool,
}

impl<'a, A, I: Iterator<Item = Option<A>>> Iterator for OptionShunt<'a, I> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        if *self.failed {
            return None;
        }
        match self.iter.next() {
            Some(Some(x)) => Some(x),
            Some(None) => {
                *self.failed = true;
                None
            }
            None => None,
        }
    }
}

/// `Result<V, E>` collects `Result<A, E>` items, stopping at the first `Err`.
impl<A, E, V: FromIterator<A>> FromIterator<Result<A, E>> for Result<V, E> {
    fn from_iter<T: IntoIterator<Item = Result<A, E>>>(iter: T) -> Result<V, E> {
        let mut error: Option<E> = None;
        let v: V = FromIterator::from_iter(ResultShunt { iter: iter.into_iter(), error: &mut error });
        match error {
            Some(e) => Err(e),
            None => Ok(v),
        }
    }
}

struct ResultShunt<'a, I, E> {
    iter: I,
    error: &'a mut Option<E>,
}

impl<'a, A, E, I: Iterator<Item = Result<A, E>>> Iterator for ResultShunt<'a, I, E> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        if self.error.is_some() {
            return None;
        }
        match self.iter.next() {
            Some(Ok(x)) => Some(x),
            Some(Err(e)) => {
                *self.error = Some(e);
                None
            }
            None => None,
        }
    }
}

pub trait Sum<A = Self>: Sized {
    fn sum<I: Iterator<Item = A>>(iter: I) -> Self;
}

pub trait Product<A = Self>: Sized {
    fn product<I: Iterator<Item = A>>(iter: I) -> Self;
}

macro_rules! sum_impl {
    ($($t:ty, $zero:expr, $one:expr;)*) => { $(
        impl Sum for $t {
            fn sum<I: Iterator<Item = $t>>(mut iter: I) -> $t {
                let mut acc: $t = $zero;
                while let Some(x) = iter.next() { acc = acc + x; }
                acc
            }
        }
        impl<'a> Sum<&'a $t> for $t {
            fn sum<I: Iterator<Item = &'a $t>>(mut iter: I) -> $t {
                let mut acc: $t = $zero;
                while let Some(x) = iter.next() { acc = acc + *x; }
                acc
            }
        }
        impl Product for $t {
            fn product<I: Iterator<Item = $t>>(mut iter: I) -> $t {
                let mut acc: $t = $one;
                while let Some(x) = iter.next() { acc = acc * x; }
                acc
            }
        }
        impl<'a> Product<&'a $t> for $t {
            fn product<I: Iterator<Item = &'a $t>>(mut iter: I) -> $t {
                let mut acc: $t = $one;
                while let Some(x) = iter.next() { acc = acc * *x; }
                acc
            }
        }
    )* };
}
sum_impl! {
    u8, 0, 1; u16, 0, 1; u32, 0, 1; u64, 0, 1; u128, 0, 1; usize, 0, 1;
    i8, 0, 1; i16, 0, 1; i32, 0, 1; i64, 0, 1; i128, 0, 1; isize, 0, 1;
    f32, 0.0, 1.0; f64, 0.0, 1.0;
}
