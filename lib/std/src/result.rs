use crate::fmt;
use crate::ops::Deref;

#[lang = "Result"]
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}

use self::Result::{Err, Ok};

impl<T, E> Result<T, E> {
    pub const fn is_ok(&self) -> bool {
        match self {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    pub const fn is_err(&self) -> bool {
        !self.is_ok()
    }
    pub fn is_ok_and<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self {
            Ok(x) => f(x),
            Err(_) => false,
        }
    }
    pub fn is_err_and<F: FnOnce(E) -> bool>(self, f: F) -> bool {
        match self {
            Ok(_) => false,
            Err(e) => f(e),
        }
    }
    pub fn ok(self) -> Option<T> {
        match self {
            Ok(x) => Some(x),
            Err(_) => None,
        }
    }
    pub fn err(self) -> Option<E> {
        match self {
            Ok(_) => None,
            Err(e) => Some(e),
        }
    }
    pub fn as_ref(&self) -> Result<&T, &E> {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(e),
        }
    }
    pub fn as_mut(&mut self) -> Result<&mut T, &mut E> {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(e),
        }
    }
    pub fn map<U, F: FnOnce(T) -> U>(self, op: F) -> Result<U, E> {
        match self {
            Ok(t) => Ok(op(t)),
            Err(e) => Err(e),
        }
    }
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U {
        match self {
            Ok(t) => f(t),
            Err(_) => default,
        }
    }
    pub fn map_or_else<U, D: FnOnce(E) -> U, F: FnOnce(T) -> U>(self, default: D, f: F) -> U {
        match self {
            Ok(t) => f(t),
            Err(e) => default(e),
        }
    }
    pub fn map_err<F, O: FnOnce(E) -> F>(self, op: O) -> Result<T, F> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(op(e)),
        }
    }
    pub fn inspect<F: FnOnce(&T)>(self, f: F) -> Result<T, E> {
        if let Ok(ref t) = self {
            f(t);
        }
        self
    }
    pub fn inspect_err<F: FnOnce(&E)>(self, f: F) -> Result<T, E> {
        if let Err(ref e) = self {
            f(e);
        }
        self
    }
    pub fn and<U>(self, res: Result<U, E>) -> Result<U, E> {
        match self {
            Ok(_) => res,
            Err(e) => Err(e),
        }
    }
    pub fn and_then<U, F: FnOnce(T) -> Result<U, E>>(self, op: F) -> Result<U, E> {
        match self {
            Ok(t) => op(t),
            Err(e) => Err(e),
        }
    }
    pub fn or<F>(self, res: Result<T, F>) -> Result<T, F> {
        match self {
            Ok(v) => Ok(v),
            Err(_) => res,
        }
    }
    pub fn or_else<F, O: FnOnce(E) -> Result<T, F>>(self, op: O) -> Result<T, F> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => op(e),
        }
    }
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Ok(t) => t,
            Err(_) => default,
        }
    }
    pub fn unwrap_or_else<F: FnOnce(E) -> T>(self, op: F) -> T {
        match self {
            Ok(t) => t,
            Err(e) => op(e),
        }
    }
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        match self {
            Ok(t) => t,
            Err(_) => T::default(),
        }
    }
    pub fn expect(self, msg: &str) -> T
    where
        E: fmt::Debug,
    {
        match self {
            Ok(t) => t,
            Err(e) => panic!("{}: {:?}", msg, e),
        }
    }
    pub fn unwrap(self) -> T
    where
        E: fmt::Debug,
    {
        match self {
            Ok(t) => t,
            Err(e) => panic!("called `Result::unwrap()` on an `Err` value: {:?}", e),
        }
    }
    pub fn expect_err(self, msg: &str) -> E
    where
        T: fmt::Debug,
    {
        match self {
            Ok(t) => panic!("{}: {:?}", msg, t),
            Err(e) => e,
        }
    }
    pub fn unwrap_err(self) -> E
    where
        T: fmt::Debug,
    {
        match self {
            Ok(t) => panic!("called `Result::unwrap_err()` on an `Ok` value: {:?}", t),
            Err(e) => e,
        }
    }
    pub fn iter(&self) -> crate::option::IntoIter<&T> {
        self.as_ref().ok().into_iter()
    }
}

impl<T: Deref, E> Result<T, E> {
    pub fn as_deref(&self) -> Result<&T::Target, &E> {
        match self {
            Ok(t) => Ok(t.deref()),
            Err(e) => Err(e),
        }
    }
}

impl<T, E> Result<&T, E> {
    pub fn copied(self) -> Result<T, E>
    where
        T: Copy,
    {
        match self {
            Ok(t) => Ok(*t),
            Err(e) => Err(e),
        }
    }
    pub fn cloned(self) -> Result<T, E>
    where
        T: Clone,
    {
        match self {
            Ok(t) => Ok(t.clone()),
            Err(e) => Err(e),
        }
    }
}

impl<T, E> Result<Option<T>, E> {
    pub fn transpose(self) -> Option<Result<T, E>> {
        match self {
            Ok(Some(x)) => Some(Ok(x)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

impl<T: Clone, E: Clone> Clone for Result<T, E> {
    fn clone(&self) -> Result<T, E> {
        match self {
            Ok(t) => Ok(t.clone()),
            Err(e) => Err(e.clone()),
        }
    }
}
impl<T: Copy, E: Copy> Copy for Result<T, E> {}

impl<T: PartialEq, E: PartialEq> PartialEq for Result<T, E> {
    fn eq(&self, other: &Result<T, E>) -> bool {
        match (self, other) {
            (Ok(a), Ok(b)) => *a == *b,
            (Err(a), Err(b)) => *a == *b,
            _ => false,
        }
    }
}
impl<T: Eq, E: Eq> Eq for Result<T, E> {}

impl<T, E> IntoIterator for Result<T, E> {
    type Item = T;
    type IntoIter = crate::option::IntoIter<T>;
    fn into_iter(self) -> crate::option::IntoIter<T> {
        self.ok().into_iter()
    }
}

impl<'a, T, E> IntoIterator for &'a Result<T, E> {
    type Item = &'a T;
    type IntoIter = crate::option::IntoIter<&'a T>;
    fn into_iter(self) -> crate::option::IntoIter<&'a T> {
        self.as_ref().ok().into_iter()
    }
}
