use crate::borrow::Borrow;
use crate::string::String;
use crate::vec::Vec;

pub trait Concat<Item: ?Sized> {
    type Output;
    fn concat(slice: &Self) -> Self::Output;
}

pub trait Join<Separator> {
    type Output;
    fn join(slice: &Self, sep: Separator) -> Self::Output;
}

impl<S: Borrow<str>> Concat<str> for [S] {
    type Output = String;
    fn concat(slice: &[S]) -> String {
        let mut out = String::new();
        let mut i = 0;
        while i < slice.len() {
            out.push_str(slice[i].borrow());
            i += 1;
        }
        out
    }
}

impl<S: Borrow<str>> Join<&str> for [S] {
    type Output = String;
    fn join(slice: &[S], sep: &str) -> String {
        let mut out = String::new();
        let mut i = 0;
        while i < slice.len() {
            if i > 0 {
                out.push_str(sep);
            }
            out.push_str(slice[i].borrow());
            i += 1;
        }
        out
    }
}

impl<T: Clone, V: Borrow<[T]>> Concat<T> for [V] {
    type Output = Vec<T>;
    fn concat(slice: &[V]) -> Vec<T> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < slice.len() {
            out.extend_from_slice(slice[i].borrow());
            i += 1;
        }
        out
    }
}

impl<T: Clone, V: Borrow<[T]>> Join<&T> for [V] {
    type Output = Vec<T>;
    fn join(slice: &[V], sep: &T) -> Vec<T> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < slice.len() {
            if i > 0 {
                out.push(sep.clone());
            }
            out.extend_from_slice(slice[i].borrow());
            i += 1;
        }
        out
    }
}

impl<T: Clone, V: Borrow<[T]>> Join<&[T]> for [V] {
    type Output = Vec<T>;
    fn join(slice: &[V], sep: &[T]) -> Vec<T> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < slice.len() {
            if i > 0 {
                out.extend_from_slice(sep);
            }
            out.extend_from_slice(slice[i].borrow());
            i += 1;
        }
        out
    }
}
