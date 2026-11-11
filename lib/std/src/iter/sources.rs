use crate::iter::traits::{DoubleEndedIterator, ExactSizeIterator, Iterator};

pub struct Empty<T> {
    marker: crate::marker::PhantomData<T>,
}
pub fn empty<T>() -> Empty<T> {
    Empty { marker: crate::marker::PhantomData }
}
impl<T> Iterator for Empty<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        None
    }
}

pub struct Once<T> {
    value: Option<T>,
}
pub fn once<T>(value: T) -> Once<T> {
    Once { value: Some(value) }
}
impl<T> Iterator for Once<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.value.take()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = if self.value.is_some() { 1 } else { 0 };
        (n, Some(n))
    }
}
impl<T> DoubleEndedIterator for Once<T> {
    fn next_back(&mut self) -> Option<T> {
        self.value.take()
    }
}
impl<T> ExactSizeIterator for Once<T> {}

pub struct OnceWith<F> {
    f: Option<F>,
}
pub fn once_with<T, F: FnOnce() -> T>(f: F) -> OnceWith<F> {
    OnceWith { f: Some(f) }
}
impl<T, F: FnOnce() -> T> Iterator for OnceWith<F> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self.f.take() {
            Some(f) => Some(f()),
            None => None,
        }
    }
}

pub struct Repeat<T> {
    value: T,
}
pub fn repeat<T: Clone>(value: T) -> Repeat<T> {
    Repeat { value }
}
impl<T: Clone> Iterator for Repeat<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        Some(self.value.clone())
    }
}

pub struct RepeatN<T> {
    value: Option<T>,
    n: usize,
}
pub fn repeat_n<T: Clone>(value: T, n: usize) -> RepeatN<T> {
    RepeatN { value: if n > 0 { Some(value) } else { None }, n }
}
impl<T: Clone> Iterator for RepeatN<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        if self.n == 0 {
            self.value.take()
        } else {
            self.value.clone()
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.n, Some(self.n))
    }
}
impl<T: Clone> ExactSizeIterator for RepeatN<T> {}
impl<T: Clone> DoubleEndedIterator for RepeatN<T> {
    fn next_back(&mut self) -> Option<T> {
        self.next()
    }
}

pub struct RepeatWith<F> {
    f: F,
}
pub fn repeat_with<T, F: FnMut() -> T>(f: F) -> RepeatWith<F> {
    RepeatWith { f }
}
impl<T, F: FnMut() -> T> Iterator for RepeatWith<F> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        Some((self.f)())
    }
}

pub struct FromFn<F> {
    f: F,
}
pub fn from_fn<T, F: FnMut() -> Option<T>>(f: F) -> FromFn<F> {
    FromFn { f }
}
impl<T, F: FnMut() -> Option<T>> Iterator for FromFn<F> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        (self.f)()
    }
}

pub struct Successors<T, F> {
    next: Option<T>,
    succ: F,
}
pub fn successors<T, F: FnMut(&T) -> Option<T>>(first: Option<T>, succ: F) -> Successors<T, F> {
    Successors { next: first, succ }
}
impl<T, F: FnMut(&T) -> Option<T>> Iterator for Successors<T, F> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        let item = match self.next.take() {
            Some(x) => x,
            None => return None,
        };
        self.next = (self.succ)(&item);
        Some(item)
    }
}
