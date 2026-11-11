#[lang = "deref"]
pub trait Deref {
    type Target: ?Sized;
    fn deref(&self) -> &Self::Target;
}

#[lang = "deref_mut"]
pub trait DerefMut: Deref {
    fn deref_mut(&mut self) -> &mut Self::Target;
}

impl<T: ?Sized> Deref for &T {
    type Target = T;
    fn deref(&self) -> &T {
        *self
    }
}

impl<T: ?Sized> Deref for &mut T {
    type Target = T;
    fn deref(&self) -> &T {
        *self
    }
}

impl<T: ?Sized> DerefMut for &mut T {
    fn deref_mut(&mut self) -> &mut T {
        *self
    }
}
