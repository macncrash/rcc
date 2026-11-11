#[lang = "From"]
pub trait From<T>: Sized {
    fn from(value: T) -> Self;
}

pub trait Into<T>: Sized {
    fn into(self) -> T;
}

impl<T, U: From<T>> Into<U> for T {
    fn into(self) -> U {
        U::from(self)
    }
}

impl<T> From<T> for T {
    fn from(t: T) -> T {
        t
    }
}

pub trait AsRef<T: ?Sized> {
    fn as_ref(&self) -> &T;
}

pub trait AsMut<T: ?Sized> {
    fn as_mut(&mut self) -> &mut T;
}

impl<T: ?Sized, U: ?Sized> AsRef<U> for &T
where
    T: AsRef<U>,
{
    fn as_ref(&self) -> &U {
        (**self).as_ref()
    }
}

impl<T: ?Sized, U: ?Sized> AsRef<U> for &mut T
where
    T: AsRef<U>,
{
    fn as_ref(&self) -> &U {
        (**self).as_ref()
    }
}

pub trait TryFrom<T>: Sized {
    type Error;
    fn try_from(value: T) -> Result<Self, Self::Error>;
}

pub trait TryInto<T>: Sized {
    type Error;
    fn try_into(self) -> Result<T, Self::Error>;
}

impl<T, U: TryFrom<T>> TryInto<U> for T {
    type Error = U::Error;
    fn try_into(self) -> Result<U, U::Error> {
        U::try_from(self)
    }
}

pub enum Infallible {}

pub fn identity<T>(x: T) -> T {
    x
}
