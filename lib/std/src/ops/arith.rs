#[lang = "add"]
pub trait Add<Rhs = Self> {
    type Output;
    fn add(self, rhs: Rhs) -> Self::Output;
}
#[lang = "sub"]
pub trait Sub<Rhs = Self> {
    type Output;
    fn sub(self, rhs: Rhs) -> Self::Output;
}
#[lang = "mul"]
pub trait Mul<Rhs = Self> {
    type Output;
    fn mul(self, rhs: Rhs) -> Self::Output;
}
#[lang = "div"]
pub trait Div<Rhs = Self> {
    type Output;
    fn div(self, rhs: Rhs) -> Self::Output;
}
#[lang = "rem"]
pub trait Rem<Rhs = Self> {
    type Output;
    fn rem(self, rhs: Rhs) -> Self::Output;
}
#[lang = "neg"]
pub trait Neg {
    type Output;
    fn neg(self) -> Self::Output;
}
#[lang = "add_assign"]
pub trait AddAssign<Rhs = Self> {
    fn add_assign(&mut self, rhs: Rhs);
}
#[lang = "sub_assign"]
pub trait SubAssign<Rhs = Self> {
    fn sub_assign(&mut self, rhs: Rhs);
}
#[lang = "mul_assign"]
pub trait MulAssign<Rhs = Self> {
    fn mul_assign(&mut self, rhs: Rhs);
}
#[lang = "div_assign"]
pub trait DivAssign<Rhs = Self> {
    fn div_assign(&mut self, rhs: Rhs);
}
#[lang = "rem_assign"]
pub trait RemAssign<Rhs = Self> {
    fn rem_assign(&mut self, rhs: Rhs);
}

// Operators on primitives are built into the compiler; these impls exist so
// that generic code (`T: Add<Output = T>`) and operators on references to
// primitives resolve.
macro_rules! arith_impl {
    ($Tr:ident $m:ident $TrA:ident $ma:ident $op:tt $opa:tt; $($t:ty)*) => { $(
        impl $Tr for $t {
            type Output = $t;
            fn $m(self, rhs: $t) -> $t { self $op rhs }
        }
        impl $Tr<&$t> for $t {
            type Output = $t;
            fn $m(self, rhs: &$t) -> $t { self $op *rhs }
        }
        impl $Tr<$t> for &$t {
            type Output = $t;
            fn $m(self, rhs: $t) -> $t { *self $op rhs }
        }
        impl $Tr<&$t> for &$t {
            type Output = $t;
            fn $m(self, rhs: &$t) -> $t { *self $op *rhs }
        }
        impl $TrA for $t {
            fn $ma(&mut self, rhs: $t) { *self $opa rhs }
        }
        impl $TrA<&$t> for $t {
            fn $ma(&mut self, rhs: &$t) { *self $opa *rhs }
        }
    )* };
}
arith_impl! { Add add AddAssign add_assign + +=; u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 }
arith_impl! { Sub sub SubAssign sub_assign - -=; u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 }
arith_impl! { Mul mul MulAssign mul_assign * *=; u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 }
arith_impl! { Div div DivAssign div_assign / /=; u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 }
arith_impl! { Rem rem RemAssign rem_assign % %=; u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 }

macro_rules! neg_impl {
    ($($t:ty)*) => { $(
        impl Neg for $t {
            type Output = $t;
            fn neg(self) -> $t { -self }
        }
        impl Neg for &$t {
            type Output = $t;
            fn neg(self) -> $t { -*self }
        }
    )* };
}
neg_impl! { i8 i16 i32 i64 i128 isize f32 f64 }
