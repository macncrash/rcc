use crate::ops::{Deref, DerefMut};

#[lang = "Option"]
pub enum Option<T> {
    None,
    Some(T),
}

use self::Option::{None, Some};

impl<T> Option<T> {
    pub const fn is_some(&self) -> bool {
        match self {
            Some(_) => true,
            None => false,
        }
    }
    pub const fn is_none(&self) -> bool {
        !self.is_some()
    }
    pub fn is_some_and<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self {
            Some(x) => f(x),
            None => false,
        }
    }
    pub fn is_none_or<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self {
            Some(x) => f(x),
            None => true,
        }
    }
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            Some(x) => Some(x),
            None => None,
        }
    }
    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            Some(x) => Some(x),
            None => None,
        }
    }
    pub fn expect(self, msg: &str) -> T {
        match self {
            Some(x) => x,
            None => panic!("{}", msg),
        }
    }
    pub fn unwrap(self) -> T {
        match self {
            Some(x) => x,
            None => panic!("called `Option::unwrap()` on a `None` value"),
        }
    }
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Some(x) => x,
            None => default,
        }
    }
    pub fn unwrap_or_else<F: FnOnce() -> T>(self, f: F) -> T {
        match self {
            Some(x) => x,
            None => f(),
        }
    }
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        match self {
            Some(x) => x,
            None => T::default(),
        }
    }
    pub unsafe fn unwrap_unchecked(self) -> T {
        match self {
            Some(x) => x,
            None => crate::intrinsics::unreachable(),
        }
    }
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Option<U> {
        match self {
            Some(x) => Some(f(x)),
            None => None,
        }
    }
    pub fn inspect<F: FnOnce(&T)>(self, f: F) -> Option<T> {
        if let Some(ref x) = self {
            f(x);
        }
        self
    }
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U {
        match self {
            Some(x) => f(x),
            None => default,
        }
    }
    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, f: F) -> U {
        match self {
            Some(x) => f(x),
            None => default(),
        }
    }
    pub fn ok_or<E>(self, err: E) -> Result<T, E> {
        match self {
            Some(x) => Ok(x),
            None => Err(err),
        }
    }
    pub fn ok_or_else<E, F: FnOnce() -> E>(self, err: F) -> Result<T, E> {
        match self {
            Some(x) => Ok(x),
            None => Err(err()),
        }
    }
    pub fn and<U>(self, optb: Option<U>) -> Option<U> {
        match self {
            Some(_) => optb,
            None => None,
        }
    }
    pub fn and_then<U, F: FnOnce(T) -> Option<U>>(self, f: F) -> Option<U> {
        match self {
            Some(x) => f(x),
            None => None,
        }
    }
    pub fn filter<P: FnOnce(&T) -> bool>(self, predicate: P) -> Option<T> {
        if let Some(x) = self {
            if predicate(&x) {
                return Some(x);
            }
        }
        None
    }
    pub fn or(self, optb: Option<T>) -> Option<T> {
        match self {
            Some(x) => Some(x),
            None => optb,
        }
    }
    pub fn or_else<F: FnOnce() -> Option<T>>(self, f: F) -> Option<T> {
        match self {
            Some(x) => Some(x),
            None => f(),
        }
    }
    pub fn xor(self, optb: Option<T>) -> Option<T> {
        match (self, optb) {
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            _ => None,
        }
    }
    pub fn insert(&mut self, value: T) -> &mut T {
        *self = Some(value);
        match self {
            Some(x) => x,
            None => crate::intrinsics::unreachable(),
        }
    }
    pub fn get_or_insert(&mut self, value: T) -> &mut T {
        if let None = *self {
            *self = Some(value);
        }
        match self {
            Some(x) => x,
            None => crate::intrinsics::unreachable(),
        }
    }
    pub fn get_or_insert_with<F: FnOnce() -> T>(&mut self, f: F) -> &mut T {
        if let None = *self {
            *self = Some(f());
        }
        match self {
            Some(x) => x,
            None => crate::intrinsics::unreachable(),
        }
    }
    pub fn take(&mut self) -> Option<T> {
        crate::mem::replace(self, None)
    }
    pub fn take_if<P: FnOnce(&mut T) -> bool>(&mut self, predicate: P) -> Option<T> {
        if self.as_mut().map_or(false, predicate) {
            self.take()
        } else {
            None
        }
    }
    pub fn replace(&mut self, value: T) -> Option<T> {
        crate::mem::replace(self, Some(value))
    }
    pub fn zip<U>(self, other: Option<U>) -> Option<(T, U)> {
        match (self, other) {
            (Some(a), Some(b)) => Some((a, b)),
            _ => None,
        }
    }
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { inner: self.as_ref() }
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut { inner: self.as_mut() }
    }
    pub fn as_slice(&self) -> &[T] {
        match self {
            Some(x) => crate::slice::from_ref(x),
            None => &[],
        }
    }
}

impl<T: Deref> Option<T> {
    pub fn as_deref(&self) -> Option<&T::Target> {
        match self {
            Some(x) => Some(x.deref()),
            None => None,
        }
    }
}

impl<T: DerefMut> Option<T> {
    pub fn as_deref_mut(&mut self) -> Option<&mut T::Target> {
        match self {
            Some(x) => Some(x.deref_mut()),
            None => None,
        }
    }
}

impl<T> Option<&T> {
    pub fn copied(self) -> Option<T>
    where
        T: Copy,
    {
        match self {
            Some(x) => Some(*x),
            None => None,
        }
    }
    pub fn cloned(self) -> Option<T>
    where
        T: Clone,
    {
        match self {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
}

impl<T> Option<&mut T> {
    pub fn copied(self) -> Option<T>
    where
        T: Copy,
    {
        match self {
            Some(x) => Some(*x),
            None => None,
        }
    }
    pub fn cloned(self) -> Option<T>
    where
        T: Clone,
    {
        match self {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
}

impl<T> Option<Option<T>> {
    pub fn flatten(self) -> Option<T> {
        match self {
            Some(x) => x,
            None => None,
        }
    }
}

impl<T, E> Option<Result<T, E>> {
    pub fn transpose(self) -> Result<Option<T>, E> {
        match self {
            Some(Ok(x)) => Ok(Some(x)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }
}

impl<T, U> Option<(T, U)> {
    pub fn unzip(self) -> (Option<T>, Option<U>) {
        match self {
            Some((a, b)) => (Some(a), Some(b)),
            None => (None, None),
        }
    }
}

impl<T: Clone> Clone for Option<T> {
    fn clone(&self) -> Option<T> {
        match self {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
}
impl<T: Copy> Copy for Option<T> {}

impl<T> Default for Option<T> {
    fn default() -> Option<T> {
        None
    }
}

impl<T: PartialEq> PartialEq for Option<T> {
    fn eq(&self, other: &Option<T>) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => *a == *b,
            (None, None) => true,
            _ => false,
        }
    }
}
impl<T: Eq> Eq for Option<T> {}

impl<T: PartialOrd> PartialOrd for Option<T> {
    fn partial_cmp(&self, other: &Option<T>) -> Option<crate::cmp::Ordering> {
        match (self, other) {
            (Some(a), Some(b)) => a.partial_cmp(b),
            (None, None) => Some(crate::cmp::Ordering::Equal),
            (None, Some(_)) => Some(crate::cmp::Ordering::Less),
            (Some(_), None) => Some(crate::cmp::Ordering::Greater),
        }
    }
}
impl<T: Ord> Ord for Option<T> {
    fn cmp(&self, other: &Option<T>) -> crate::cmp::Ordering {
        match (self, other) {
            (Some(a), Some(b)) => a.cmp(b),
            (None, None) => crate::cmp::Ordering::Equal,
            (None, Some(_)) => crate::cmp::Ordering::Less,
            (Some(_), None) => crate::cmp::Ordering::Greater,
        }
    }
}

impl<T> From<T> for Option<T> {
    fn from(t: T) -> Option<T> {
        Some(t)
    }
}

pub struct Iter<'a, T> {
    inner: Option<&'a T>,
}
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        self.inner.take()
    }
}
pub struct IterMut<'a, T> {
    inner: Option<&'a mut T>,
}
impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T> {
        self.inner.take()
    }
}
pub struct IntoIter<T> {
    inner: Option<T>,
}
impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.inner.take()
    }
}
impl<T> IntoIterator for Option<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { inner: self }
    }
}
impl<'a, T> IntoIterator for &'a Option<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Option<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}
