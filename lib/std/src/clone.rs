#[lang = "clone"]
pub trait Clone: Sized {
    fn clone(&self) -> Self;
    fn clone_from(&mut self, source: &Self) {
        *self = source.clone()
    }
}

macro_rules! impl_clone {
    ($($t:ty)*) => { $( impl Clone for $t { fn clone(&self) -> $t { *self } } )* };
}
impl_clone! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 bool char }

impl<T: ?Sized> Clone for &T {
    fn clone(&self) -> &T {
        *self
    }
}
impl<T: ?Sized> Clone for *const T {
    fn clone(&self) -> *const T {
        *self
    }
}
impl<T: ?Sized> Clone for *mut T {
    fn clone(&self) -> *mut T {
        *self
    }
}
impl Clone for () {
    fn clone(&self) {}
}

macro_rules! tuple_clone {
    ($( ($($name:ident $idx:tt)+) )*) => { $(
        impl<$($name: Clone),+> Clone for ($($name,)+) {
            fn clone(&self) -> ($($name,)+) { ($(self.$idx.clone(),)+) }
        }
    )* };
}
tuple_clone! {
    (A 0) (A 0 B 1) (A 0 B 1 C 2) (A 0 B 1 C 2 D 3) (A 0 B 1 C 2 D 3 E 4)
    (A 0 B 1 C 2 D 3 E 4 F 5) (A 0 B 1 C 2 D 3 E 4 F 5 G 6) (A 0 B 1 C 2 D 3 E 4 F 5 G 6 H 7)
}
