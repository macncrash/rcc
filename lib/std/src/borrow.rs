pub trait Borrow<Borrowed: ?Sized> {
    fn borrow(&self) -> &Borrowed;
}
pub trait BorrowMut<Borrowed: ?Sized>: Borrow<Borrowed> {
    fn borrow_mut(&mut self) -> &mut Borrowed;
}

impl<T: ?Sized> Borrow<T> for T {
    fn borrow(&self) -> &T {
        self
    }
}
impl<T: ?Sized> BorrowMut<T> for T {
    fn borrow_mut(&mut self) -> &mut T {
        self
    }
}
impl<T: ?Sized> Borrow<T> for &T {
    fn borrow(&self) -> &T {
        &**self
    }
}
impl<T: ?Sized> Borrow<T> for &mut T {
    fn borrow(&self) -> &T {
        &**self
    }
}

pub trait ToOwned {
    type Owned: Borrow<Self>;
    fn to_owned(&self) -> Self::Owned;
}

impl<T: Clone> ToOwned for T {
    type Owned = T;
    fn to_owned(&self) -> T {
        self.clone()
    }
}

/// Clone-on-write pointer.
pub enum Cow<'a, B: ?Sized + ToOwned + 'a> {
    Borrowed(&'a B),
    Owned(<B as ToOwned>::Owned),
}

impl<'a, B: ?Sized + ToOwned> Cow<'a, B> {
    pub fn into_owned(self) -> <B as ToOwned>::Owned {
        match self {
            Cow::Borrowed(b) => b.to_owned(),
            Cow::Owned(o) => o,
        }
    }
    pub fn is_borrowed(&self) -> bool {
        match self {
            Cow::Borrowed(_) => true,
            Cow::Owned(_) => false,
        }
    }
    pub fn is_owned(&self) -> bool {
        !self.is_borrowed()
    }
    pub fn to_mut(&mut self) -> &mut <B as ToOwned>::Owned {
        if let Cow::Borrowed(b) = *self {
            *self = Cow::Owned(b.to_owned());
        }
        match self {
            Cow::Owned(o) => o,
            Cow::Borrowed(_) => crate::intrinsics::unreachable(),
        }
    }
}

impl<'a, B: ?Sized + ToOwned> crate::ops::Deref for Cow<'a, B> {
    type Target = B;
    fn deref(&self) -> &B {
        match self {
            Cow::Borrowed(b) => *b,
            Cow::Owned(o) => o.borrow(),
        }
    }
}

impl<'a, B: ?Sized + ToOwned> Clone for Cow<'a, B> {
    fn clone(&self) -> Cow<'a, B> {
        match self {
            Cow::Borrowed(b) => Cow::Borrowed(*b),
            Cow::Owned(o) => {
                let b: &B = o.borrow();
                Cow::Owned(b.to_owned())
            }
        }
    }
}

impl<'a, B: ?Sized + ToOwned + crate::fmt::Display> crate::fmt::Display for Cow<'a, B> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        crate::fmt::Display::fmt(&**self, f)
    }
}
impl<'a, B: ?Sized + ToOwned + crate::fmt::Debug> crate::fmt::Debug for Cow<'a, B> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        crate::fmt::Debug::fmt(&**self, f)
    }
}
impl<'a, 'b, B: ?Sized + ToOwned + PartialEq> PartialEq<Cow<'b, B>> for Cow<'a, B> {
    fn eq(&self, other: &Cow<'b, B>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}
impl<'a> PartialEq<str> for Cow<'a, str> {
    fn eq(&self, other: &str) -> bool {
        &**self == other
    }
}
impl<'a> PartialEq<&str> for Cow<'a, str> {
    fn eq(&self, other: &&str) -> bool {
        &**self == *other
    }
}
impl<'a> From<&'a str> for Cow<'a, str> {
    fn from(s: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(s)
    }
}
impl<'a> From<crate::string::String> for Cow<'a, str> {
    fn from(s: crate::string::String) -> Cow<'a, str> {
        Cow::Owned(s)
    }
}
impl<'a, B: ?Sized + ToOwned> AsRef<B> for Cow<'a, B> {
    fn as_ref(&self) -> &B {
        &**self
    }
}
