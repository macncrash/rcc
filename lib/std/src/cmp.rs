use crate::option::Option::{self, None, Some};

#[lang = "eq"]
pub trait PartialEq<Rhs: ?Sized = Self> {
    fn eq(&self, other: &Rhs) -> bool;
    fn ne(&self, other: &Rhs) -> bool {
        !self.eq(other)
    }
}

pub trait Eq: PartialEq<Self> {}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Ordering {
    Less = -1,
    Equal = 0,
    Greater = 1,
}

impl Ordering {
    pub fn is_eq(self) -> bool {
        matches!(self, Ordering::Equal)
    }
    pub fn is_ne(self) -> bool {
        !matches!(self, Ordering::Equal)
    }
    pub fn is_lt(self) -> bool {
        matches!(self, Ordering::Less)
    }
    pub fn is_gt(self) -> bool {
        matches!(self, Ordering::Greater)
    }
    pub fn is_le(self) -> bool {
        !matches!(self, Ordering::Greater)
    }
    pub fn is_ge(self) -> bool {
        !matches!(self, Ordering::Less)
    }
    pub fn reverse(self) -> Ordering {
        match self {
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => Ordering::Equal,
            Ordering::Greater => Ordering::Less,
        }
    }
    pub fn then(self, other: Ordering) -> Ordering {
        match self {
            Ordering::Equal => other,
            _ => self,
        }
    }
    pub fn then_with<F: FnOnce() -> Ordering>(self, f: F) -> Ordering {
        match self {
            Ordering::Equal => f(),
            _ => self,
        }
    }
}

impl PartialOrd for Ordering {
    fn partial_cmp(&self, other: &Ordering) -> Option<Ordering> {
        (*self as i32).partial_cmp(&(*other as i32))
    }
}
impl Ord for Ordering {
    fn cmp(&self, other: &Ordering) -> Ordering {
        (*self as i32).cmp(&(*other as i32))
    }
}

#[lang = "partial_ord"]
pub trait PartialOrd<Rhs: ?Sized = Self>: PartialEq<Rhs> {
    fn partial_cmp(&self, other: &Rhs) -> Option<Ordering>;
    fn lt(&self, other: &Rhs) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Less))
    }
    fn le(&self, other: &Rhs) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Less | Ordering::Equal))
    }
    fn gt(&self, other: &Rhs) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Greater))
    }
    fn ge(&self, other: &Rhs) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Greater | Ordering::Equal))
    }
}

pub trait Ord: Eq + PartialOrd<Self> {
    fn cmp(&self, other: &Self) -> Ordering;
    fn max(self, other: Self) -> Self
    where
        Self: Sized,
    {
        if other < self { self } else { other }
    }
    fn min(self, other: Self) -> Self
    where
        Self: Sized,
    {
        if other < self { other } else { self }
    }
    fn clamp(self, min: Self, max: Self) -> Self
    where
        Self: Sized,
    {
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }
}

pub fn min<T: Ord>(a: T, b: T) -> T {
    a.min(b)
}
pub fn max<T: Ord>(a: T, b: T) -> T {
    a.max(b)
}
pub fn min_by_key<T, K: Ord, F: FnMut(&T) -> K>(a: T, b: T, mut f: F) -> T {
    if f(&b) < f(&a) { b } else { a }
}
pub fn max_by_key<T, K: Ord, F: FnMut(&T) -> K>(a: T, b: T, mut f: F) -> T {
    if f(&b) < f(&a) { a } else { b }
}

macro_rules! int_cmp {
    ($($t:ty)*) => { $(
        impl PartialEq for $t {
            fn eq(&self, other: &$t) -> bool { *self == *other }
            fn ne(&self, other: &$t) -> bool { *self != *other }
        }
        impl Eq for $t {}
        impl PartialOrd for $t {
            fn partial_cmp(&self, other: &$t) -> Option<Ordering> {
                Some(if *self < *other { Ordering::Less } else if *self == *other { Ordering::Equal } else { Ordering::Greater })
            }
            fn lt(&self, other: &$t) -> bool { *self < *other }
            fn le(&self, other: &$t) -> bool { *self <= *other }
            fn gt(&self, other: &$t) -> bool { *self > *other }
            fn ge(&self, other: &$t) -> bool { *self >= *other }
        }
        impl Ord for $t {
            fn cmp(&self, other: &$t) -> Ordering {
                if *self < *other { Ordering::Less } else if *self == *other { Ordering::Equal } else { Ordering::Greater }
            }
        }
    )* };
}
int_cmp! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize bool char }

macro_rules! float_cmp {
    ($($t:ty)*) => { $(
        impl PartialEq for $t {
            fn eq(&self, other: &$t) -> bool { *self == *other }
            fn ne(&self, other: &$t) -> bool { *self != *other }
        }
        impl PartialOrd for $t {
            fn partial_cmp(&self, other: &$t) -> Option<Ordering> {
                if *self < *other { Some(Ordering::Less) }
                else if *self == *other { Some(Ordering::Equal) }
                else if *self > *other { Some(Ordering::Greater) }
                else { None }
            }
            fn lt(&self, other: &$t) -> bool { *self < *other }
            fn le(&self, other: &$t) -> bool { *self <= *other }
            fn gt(&self, other: &$t) -> bool { *self > *other }
            fn ge(&self, other: &$t) -> bool { *self >= *other }
        }
    )* };
}
float_cmp! { f32 f64 }

impl PartialEq for () {
    fn eq(&self, _other: &()) -> bool {
        true
    }
}
impl Eq for () {}
impl PartialOrd for () {
    fn partial_cmp(&self, _other: &()) -> Option<Ordering> {
        Some(Ordering::Equal)
    }
}
impl Ord for () {
    fn cmp(&self, _other: &()) -> Ordering {
        Ordering::Equal
    }
}

// References compare by value.
impl<A: ?Sized + PartialEq<B>, B: ?Sized> PartialEq<&B> for &A {
    fn eq(&self, other: &&B) -> bool {
        PartialEq::eq(*self, *other)
    }
}
impl<A: ?Sized + PartialEq<B>, B: ?Sized> PartialEq<&mut B> for &mut A {
    fn eq(&self, other: &&mut B) -> bool {
        PartialEq::eq(*self, *other)
    }
}
impl<A: ?Sized + PartialEq<B>, B: ?Sized> PartialEq<&mut B> for &A {
    fn eq(&self, other: &&mut B) -> bool {
        PartialEq::eq(*self, *other)
    }
}
impl<A: ?Sized + PartialEq<B>, B: ?Sized> PartialEq<&B> for &mut A {
    fn eq(&self, other: &&B) -> bool {
        PartialEq::eq(*self, *other)
    }
}
impl<A: ?Sized + Eq> Eq for &A {}
impl<A: ?Sized + Eq> Eq for &mut A {}
impl<A: ?Sized + PartialOrd<B>, B: ?Sized> PartialOrd<&B> for &A {
    fn partial_cmp(&self, other: &&B) -> Option<Ordering> {
        PartialOrd::partial_cmp(*self, *other)
    }
}
impl<A: ?Sized + Ord> Ord for &A {
    fn cmp(&self, other: &&A) -> Ordering {
        Ord::cmp(*self, *other)
    }
}

macro_rules! tuple_cmp {
    ($( ($($name:ident $idx:tt)+) )*) => { $(
        impl<$($name: PartialEq),+> PartialEq for ($($name,)+) {
            fn eq(&self, other: &($($name,)+)) -> bool { $(self.$idx == other.$idx)&&+ }
        }
        impl<$($name: Eq),+> Eq for ($($name,)+) {}
        impl<$($name: PartialOrd),+> PartialOrd for ($($name,)+) {
            fn partial_cmp(&self, other: &($($name,)+)) -> Option<Ordering> {
                $(
                    match self.$idx.partial_cmp(&other.$idx) {
                        Some(Ordering::Equal) => {}
                        r => return r,
                    }
                )+
                Some(Ordering::Equal)
            }
        }
        impl<$($name: Ord),+> Ord for ($($name,)+) {
            fn cmp(&self, other: &($($name,)+)) -> Ordering {
                $(
                    match self.$idx.cmp(&other.$idx) {
                        Ordering::Equal => {}
                        r => return r,
                    }
                )+
                Ordering::Equal
            }
        }
    )* };
}
tuple_cmp! {
    (A 0) (A 0 B 1) (A 0 B 1 C 2) (A 0 B 1 C 2 D 3) (A 0 B 1 C 2 D 3 E 4)
    (A 0 B 1 C 2 D 3 E 4 F 5) (A 0 B 1 C 2 D 3 E 4 F 5 G 6) (A 0 B 1 C 2 D 3 E 4 F 5 G 6 H 7)
}

/// Reverses the ordering of the wrapped value.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub struct Reverse<T>(pub T);

impl<T: PartialOrd> PartialOrd for Reverse<T> {
    fn partial_cmp(&self, other: &Reverse<T>) -> Option<Ordering> {
        other.0.partial_cmp(&self.0)
    }
}
impl<T: Ord> Ord for Reverse<T> {
    fn cmp(&self, other: &Reverse<T>) -> Ordering {
        other.0.cmp(&self.0)
    }
}
