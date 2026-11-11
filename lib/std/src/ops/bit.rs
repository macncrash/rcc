#[lang = "not"]
pub trait Not {
    type Output;
    fn not(self) -> Self::Output;
}
#[lang = "bitand"]
pub trait BitAnd<Rhs = Self> {
    type Output;
    fn bitand(self, rhs: Rhs) -> Self::Output;
}
#[lang = "bitor"]
pub trait BitOr<Rhs = Self> {
    type Output;
    fn bitor(self, rhs: Rhs) -> Self::Output;
}
#[lang = "bitxor"]
pub trait BitXor<Rhs = Self> {
    type Output;
    fn bitxor(self, rhs: Rhs) -> Self::Output;
}
#[lang = "shl"]
pub trait Shl<Rhs = Self> {
    type Output;
    fn shl(self, rhs: Rhs) -> Self::Output;
}
#[lang = "shr"]
pub trait Shr<Rhs = Self> {
    type Output;
    fn shr(self, rhs: Rhs) -> Self::Output;
}
#[lang = "bitand_assign"]
pub trait BitAndAssign<Rhs = Self> {
    fn bitand_assign(&mut self, rhs: Rhs);
}
#[lang = "bitor_assign"]
pub trait BitOrAssign<Rhs = Self> {
    fn bitor_assign(&mut self, rhs: Rhs);
}
#[lang = "bitxor_assign"]
pub trait BitXorAssign<Rhs = Self> {
    fn bitxor_assign(&mut self, rhs: Rhs);
}
#[lang = "shl_assign"]
pub trait ShlAssign<Rhs = Self> {
    fn shl_assign(&mut self, rhs: Rhs);
}
#[lang = "shr_assign"]
pub trait ShrAssign<Rhs = Self> {
    fn shr_assign(&mut self, rhs: Rhs);
}

macro_rules! bit_impl {
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
bit_impl! { BitAnd bitand BitAndAssign bitand_assign & &=; bool u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize }
bit_impl! { BitOr bitor BitOrAssign bitor_assign | |=; bool u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize }
bit_impl! { BitXor bitxor BitXorAssign bitxor_assign ^ ^=; bool u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize }

macro_rules! not_impl {
    ($($t:ty)*) => { $(
        impl Not for $t {
            type Output = $t;
            fn not(self) -> $t { !self }
        }
        impl Not for &$t {
            type Output = $t;
            fn not(self) -> $t { !*self }
        }
    )* };
}
not_impl! { bool u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize }

// Shifts by the common amount types.
macro_rules! shift_impl {
    ($($t:ty)*) => { $(
        shift_impl_one! { $t; u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize }
    )* };
}
macro_rules! shift_impl_one {
    ($t:ty; $($r:ty)*) => { $(
        impl Shl<$r> for $t {
            type Output = $t;
            fn shl(self, rhs: $r) -> $t { self << rhs }
        }
        impl Shr<$r> for $t {
            type Output = $t;
            fn shr(self, rhs: $r) -> $t { self >> rhs }
        }
        impl Shl<$r> for &$t {
            type Output = $t;
            fn shl(self, rhs: $r) -> $t { *self << rhs }
        }
        impl Shr<$r> for &$t {
            type Output = $t;
            fn shr(self, rhs: $r) -> $t { *self >> rhs }
        }
        impl ShlAssign<$r> for $t {
            fn shl_assign(&mut self, rhs: $r) { *self <<= rhs }
        }
        impl ShrAssign<$r> for $t {
            fn shr_assign(&mut self, rhs: $r) { *self >>= rhs }
        }
    )* };
}
shift_impl! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize }
