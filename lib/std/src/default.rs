pub trait Default: Sized {
    fn default() -> Self;
}

macro_rules! impl_default {
    ($($t:ty, $v:expr;)*) => { $( impl Default for $t { fn default() -> $t { $v } } )* };
}
impl_default! {
    u8, 0; u16, 0; u32, 0; u64, 0; u128, 0; usize, 0;
    i8, 0; i16, 0; i32, 0; i64, 0; i128, 0; isize, 0;
    f32, 0.0; f64, 0.0; bool, false; char, '\0';
}

impl Default for () {
    fn default() {}
}
impl Default for &str {
    fn default() -> &'static str {
        ""
    }
}

macro_rules! tuple_default {
    ($( ($($name:ident)+) )*) => { $(
        impl<$($name: Default),+> Default for ($($name,)+) {
            fn default() -> ($($name,)+) { ($($name::default(),)+) }
        }
    )* };
}
tuple_default! { (A) (A B) (A B C) (A B C D) (A B C D E) (A B C D E F) }
