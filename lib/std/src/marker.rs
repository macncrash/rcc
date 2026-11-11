#[lang = "sized"]
pub trait Sized {}

#[lang = "copy"]
pub trait Copy: Clone {}

#[rcc_auto]
pub trait Send {}
#[rcc_auto]
pub trait Sync {}
#[rcc_auto]
pub trait Unpin {}

#[lang = "phantom_data"]
pub struct PhantomData<T: ?Sized>;

impl<T: ?Sized> Clone for PhantomData<T> {
    fn clone(&self) -> PhantomData<T> {
        PhantomData
    }
}
impl<T: ?Sized> Copy for PhantomData<T> {}
impl<T: ?Sized> Default for PhantomData<T> {
    fn default() -> PhantomData<T> {
        PhantomData
    }
}
impl<T: ?Sized> PartialEq for PhantomData<T> {
    fn eq(&self, _other: &PhantomData<T>) -> bool {
        true
    }
}
impl<T: ?Sized> Eq for PhantomData<T> {}

macro_rules! impl_copy {
    ($($t:ty)*) => { $( impl Copy for $t {} )* };
}
impl_copy! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 bool char }

impl<T: ?Sized> Copy for &T {}
impl<T: ?Sized> Copy for *const T {}
impl<T: ?Sized> Copy for *mut T {}
impl Copy for () {}
impl<T: Copy, const N: usize> Copy for [T; N] {}

macro_rules! tuple_copy {
    ($( ($($name:ident)+) )*) => { $( impl<$($name: Copy),+> Copy for ($($name,)+) {} )* };
}
tuple_copy! { (A) (A B) (A B C) (A B C D) (A B C D E) (A B C D E F) (A B C D E F G) (A B C D E F G H) }
