use crate::intrinsics;
use crate::num::TryFromIntError;

macro_rules! int_common {
    ($t:ident, $ut:ident, $bits:expr, $min:expr, $max:expr) => {
        pub const MIN: $t = $min;
        pub const MAX: $t = $max;
        pub const BITS: u32 = $bits;

        pub const fn wrapping_add(self, rhs: $t) -> $t { self + rhs }
        pub const fn wrapping_sub(self, rhs: $t) -> $t { self - rhs }
        pub const fn wrapping_mul(self, rhs: $t) -> $t { self * rhs }
        pub const fn wrapping_shl(self, rhs: u32) -> $t { self << rhs }
        pub const fn wrapping_shr(self, rhs: u32) -> $t { self >> rhs }
        pub fn wrapping_pow(self, mut exp: u32) -> $t {
            let mut base = self;
            let mut acc: $t = 1;
            while exp > 0 {
                if exp & 1 == 1 {
                    acc = acc * base;
                }
                exp >>= 1;
                if exp > 0 {
                    base = base * base;
                }
            }
            acc
        }
        pub fn pow(self, exp: u32) -> $t { self.wrapping_pow(exp) }

        pub fn overflowing_add(self, rhs: $t) -> ($t, bool) { unsafe { intrinsics::add_with_overflow(self, rhs) } }
        pub fn overflowing_sub(self, rhs: $t) -> ($t, bool) { unsafe { intrinsics::sub_with_overflow(self, rhs) } }
        pub fn overflowing_mul(self, rhs: $t) -> ($t, bool) { unsafe { intrinsics::mul_with_overflow(self, rhs) } }
        pub fn checked_add(self, rhs: $t) -> Option<$t> {
            let (r, o) = self.overflowing_add(rhs);
            if o { None } else { Some(r) }
        }
        pub fn checked_sub(self, rhs: $t) -> Option<$t> {
            let (r, o) = self.overflowing_sub(rhs);
            if o { None } else { Some(r) }
        }
        pub fn checked_mul(self, rhs: $t) -> Option<$t> {
            let (r, o) = self.overflowing_mul(rhs);
            if o { None } else { Some(r) }
        }
        pub fn checked_shl(self, rhs: u32) -> Option<$t> {
            if rhs < $bits { Some(self << rhs) } else { None }
        }
        pub fn checked_shr(self, rhs: u32) -> Option<$t> {
            if rhs < $bits { Some(self >> rhs) } else { None }
        }
        pub fn overflowing_shl(self, rhs: u32) -> ($t, bool) { (self << rhs, rhs >= $bits) }
        pub fn overflowing_shr(self, rhs: u32) -> ($t, bool) { (self >> rhs, rhs >= $bits) }
        pub fn checked_pow(self, mut exp: u32) -> Option<$t> {
            let mut base = self;
            let mut acc: $t = 1;
            while exp > 0 {
                if exp & 1 == 1 {
                    acc = match acc.checked_mul(base) {
                        Some(v) => v,
                        None => return None,
                    };
                }
                exp >>= 1;
                if exp > 0 {
                    base = match base.checked_mul(base) {
                        Some(v) => v,
                        None => return None,
                    };
                }
            }
            Some(acc)
        }
        pub fn saturating_pow(self, exp: u32) -> $t {
            match self.checked_pow(exp) {
                Some(v) => v,
                None => if (self as i128) < 0 && exp % 2 == 1 { <$t>::MIN } else { <$t>::MAX },
            }
        }

        pub fn count_ones(self) -> u32 { unsafe { intrinsics::ctpop(self) } }
        pub fn count_zeros(self) -> u32 { $bits - self.count_ones() }
        pub fn leading_zeros(self) -> u32 { unsafe { intrinsics::ctlz(self) } }
        pub fn trailing_zeros(self) -> u32 { unsafe { intrinsics::cttz(self) } }
        pub fn leading_ones(self) -> u32 { (!self).leading_zeros() }
        pub fn trailing_ones(self) -> u32 { (!self).trailing_zeros() }
        pub fn swap_bytes(self) -> $t { unsafe { intrinsics::bswap(self) } }
        pub fn reverse_bits(self) -> $t {
            let mut x = self as $ut;
            let mut r: $ut = 0;
            let mut i = 0;
            while i < $bits {
                r = (r << 1) | (x & 1);
                x >>= 1;
                i += 1;
            }
            r as $t
        }
        pub fn rotate_left(self, n: u32) -> $t { unsafe { intrinsics::rotate_left(self, n) } }
        pub fn rotate_right(self, n: u32) -> $t { unsafe { intrinsics::rotate_right(self, n) } }
        pub fn to_be(self) -> $t { self.swap_bytes() }
        pub fn to_le(self) -> $t { self }
        pub fn from_be(x: $t) -> $t { x.swap_bytes() }
        pub fn from_le(x: $t) -> $t { x }

        pub fn to_ne_bytes(self) -> [u8; $bits / 8] { unsafe { intrinsics::transmute(self) } }
        pub fn to_le_bytes(self) -> [u8; $bits / 8] { unsafe { intrinsics::transmute(self) } }
        pub fn to_be_bytes(self) -> [u8; $bits / 8] { unsafe { intrinsics::transmute(self.swap_bytes()) } }
        pub fn from_ne_bytes(bytes: [u8; $bits / 8]) -> $t { unsafe { intrinsics::transmute(bytes) } }
        pub fn from_le_bytes(bytes: [u8; $bits / 8]) -> $t { unsafe { intrinsics::transmute(bytes) } }
        pub fn from_be_bytes(bytes: [u8; $bits / 8]) -> $t {
            let x: $t = unsafe { intrinsics::transmute(bytes) };
            x.swap_bytes()
        }

        pub fn from_str_radix(src: &str, radix: u32) -> Result<$t, crate::num::ParseIntError> {
            match crate::num::parse::parse_int(src.as_bytes(), radix, ($min as i128) < 0, $max as u128) {
                Ok(v) => Ok(v as $t),
                Err(e) => Err(e),
            }
        }
    };
}

macro_rules! uint_impl {
    ($t:ident, $st:ident, $bits:expr, $max:expr) => {
        impl $t {
            int_common!($t, $t, $bits, 0, $max);

            pub const fn wrapping_neg(self) -> $t { (0 as $t) - self }
            pub fn wrapping_div(self, rhs: $t) -> $t { self / rhs }
            pub fn wrapping_rem(self, rhs: $t) -> $t { self % rhs }
            pub fn checked_div(self, rhs: $t) -> Option<$t> {
                if rhs == 0 { None } else { Some(self / rhs) }
            }
            pub fn checked_rem(self, rhs: $t) -> Option<$t> {
                if rhs == 0 { None } else { Some(self % rhs) }
            }
            pub fn checked_neg(self) -> Option<$t> {
                if self == 0 { Some(0) } else { None }
            }
            pub fn saturating_add(self, rhs: $t) -> $t {
                let (r, o) = self.overflowing_add(rhs);
                if o { $max } else { r }
            }
            pub fn saturating_sub(self, rhs: $t) -> $t {
                if self < rhs { 0 } else { self - rhs }
            }
            pub fn saturating_mul(self, rhs: $t) -> $t {
                let (r, o) = self.overflowing_mul(rhs);
                if o { $max } else { r }
            }
            pub fn abs_diff(self, other: $t) -> $t {
                if self < other { other - self } else { self - other }
            }
            pub fn div_euclid(self, rhs: $t) -> $t { self / rhs }
            pub fn rem_euclid(self, rhs: $t) -> $t { self % rhs }
            pub fn wrapping_div_euclid(self, rhs: $t) -> $t { self / rhs }
            pub fn wrapping_rem_euclid(self, rhs: $t) -> $t { self % rhs }
            pub fn checked_div_euclid(self, rhs: $t) -> Option<$t> { if rhs == 0 { None } else { Some(self / rhs) } }
            pub fn checked_rem_euclid(self, rhs: $t) -> Option<$t> { if rhs == 0 { None } else { Some(self % rhs) } }
            pub fn overflowing_neg(self) -> ($t, bool) { ((0 as $t) - self, self != 0) }
            pub fn overflowing_div(self, rhs: $t) -> ($t, bool) { (self / rhs, false) }
            pub fn overflowing_rem(self, rhs: $t) -> ($t, bool) { (self % rhs, false) }
            pub fn saturating_div(self, rhs: $t) -> $t { self / rhs }
            pub fn overflowing_pow(self, exp: u32) -> ($t, bool) {
                match self.checked_pow(exp) {
                    Some(v) => (v, false),
                    None => (self.wrapping_pow(exp), true),
                }
            }
            pub fn checked_ilog2(self) -> Option<u32> { if self == 0 { None } else { Some(self.ilog2()) } }
            pub fn checked_next_multiple_of(self, rhs: $t) -> Option<$t> {
                if rhs == 0 { return None; }
                let r = self % rhs;
                if r == 0 { Some(self) } else { self.checked_add(rhs - r) }
            }
            pub fn div_ceil(self, rhs: $t) -> $t {
                let d = self / rhs;
                let r = self % rhs;
                if r > 0 { d + 1 } else { d }
            }
            pub fn next_multiple_of(self, rhs: $t) -> $t {
                let r = self % rhs;
                if r == 0 { self } else { self + (rhs - r) }
            }
            pub fn is_multiple_of(self, rhs: $t) -> bool {
                if rhs == 0 { self == 0 } else { self % rhs == 0 }
            }
            pub fn is_power_of_two(self) -> bool { self.count_ones() == 1 }
            pub fn next_power_of_two(self) -> $t {
                if self <= 1 {
                    return 1;
                }
                let p = self - 1;
                (1 as $t) << ($bits - p.leading_zeros())
            }
            pub fn checked_next_power_of_two(self) -> Option<$t> {
                if self <= 1 {
                    return Some(1);
                }
                let z = (self - 1).leading_zeros();
                if z == 0 { None } else { Some((1 as $t) << ($bits - z)) }
            }
            pub fn ilog2(self) -> u32 { $bits - 1 - self.leading_zeros() }
            pub fn ilog10(self) -> u32 {
                let mut n = 0;
                let mut x = self;
                while x >= 10 {
                    x /= 10;
                    n += 1;
                }
                n
            }
            pub fn isqrt(self) -> $t {
                if self < 2 {
                    return self;
                }
                let mut lo: $t = 1;
                let mut hi: $t = (1 as $t) << (($bits - self.leading_zeros() + 1) / 2);
                while lo < hi {
                    let mid = lo + (hi - lo + 1) / 2;
                    if mid <= self / mid { lo = mid } else { hi = mid - 1 }
                }
                lo
            }
            pub fn min_value() -> $t { 0 }
            pub fn max_value() -> $t { $max }
            pub fn cast_signed(self) -> $st { self as $st }
            pub fn saturating_sub_signed(self, rhs: $st) -> $t {
                if rhs >= 0 { self.saturating_sub(rhs as $t) } else { self.saturating_add((0 as $t) - (rhs as $t)) }
            }
            pub fn wrapping_add_signed(self, rhs: $st) -> $t { self + (rhs as $t) }
            pub fn checked_add_signed(self, rhs: $st) -> Option<$t> {
                let r = self + (rhs as $t);
                if rhs >= 0 {
                    if r < self { None } else { Some(r) }
                } else if r > self {
                    None
                } else {
                    Some(r)
                }
            }
            pub fn saturating_add_signed(self, rhs: $st) -> $t {
                match self.checked_add_signed(rhs) {
                    Some(v) => v,
                    None => if rhs < 0 { 0 } else { $max },
                }
            }
        }
    };
}

macro_rules! sint_impl {
    ($t:ident, $ut:ident, $bits:expr, $min:expr, $max:expr) => {
        impl $t {
            int_common!($t, $ut, $bits, $min, $max);

            pub const fn wrapping_neg(self) -> $t { (0 as $t) - self }
            pub fn wrapping_abs(self) -> $t { if self < 0 { (0 as $t) - self } else { self } }
            pub fn wrapping_div(self, rhs: $t) -> $t {
                if rhs == -1 { (0 as $t) - self } else { self / rhs }
            }
            pub fn wrapping_rem(self, rhs: $t) -> $t {
                if rhs == -1 { 0 } else { self % rhs }
            }
            pub fn checked_div(self, rhs: $t) -> Option<$t> {
                if rhs == 0 || (self == $min && rhs == -1) { None } else { Some(self / rhs) }
            }
            pub fn checked_rem(self, rhs: $t) -> Option<$t> {
                if rhs == 0 || (self == $min && rhs == -1) { None } else { Some(self % rhs) }
            }
            pub fn checked_neg(self) -> Option<$t> {
                if self == $min { None } else { Some(-self) }
            }
            pub fn checked_abs(self) -> Option<$t> {
                if self == $min { None } else { Some(self.wrapping_abs()) }
            }
            pub fn overflowing_neg(self) -> ($t, bool) { ((0 as $t) - self, self == $min) }
            pub fn saturating_add(self, rhs: $t) -> $t {
                let (r, o) = self.overflowing_add(rhs);
                if !o { r } else if rhs < 0 { $min } else { $max }
            }
            pub fn saturating_sub(self, rhs: $t) -> $t {
                let (r, o) = self.overflowing_sub(rhs);
                if !o { r } else if rhs < 0 { $max } else { $min }
            }
            pub fn saturating_mul(self, rhs: $t) -> $t {
                let (r, o) = self.overflowing_mul(rhs);
                if !o { r } else if (self < 0) == (rhs < 0) { $max } else { $min }
            }
            pub fn saturating_neg(self) -> $t { if self == $min { $max } else { -self } }
            pub fn saturating_abs(self) -> $t { if self == $min { $max } else { self.wrapping_abs() } }
            pub fn abs(self) -> $t { if self < 0 { (0 as $t) - self } else { self } }
            pub fn unsigned_abs(self) -> $ut { self.wrapping_abs() as $ut }
            pub fn abs_diff(self, other: $t) -> $ut {
                if self < other { (other as $ut) - (self as $ut) } else { (self as $ut) - (other as $ut) }
            }
            pub fn signum(self) -> $t {
                if self < 0 { -1 } else if self == 0 { 0 } else { 1 }
            }
            pub fn is_negative(self) -> bool { self < 0 }
            pub fn is_positive(self) -> bool { self > 0 }
            pub fn div_euclid(self, rhs: $t) -> $t {
                let q = self / rhs;
                if self % rhs < 0 {
                    if rhs > 0 { q - 1 } else { q + 1 }
                } else {
                    q
                }
            }
            pub fn rem_euclid(self, rhs: $t) -> $t {
                let r = self % rhs;
                if r < 0 { r + rhs.wrapping_abs() } else { r }
            }
            pub fn wrapping_div_euclid(self, rhs: $t) -> $t {
                if rhs == -1 { (0 as $t) - self } else { self.div_euclid(rhs) }
            }
            pub fn wrapping_rem_euclid(self, rhs: $t) -> $t {
                if rhs == -1 { 0 } else { self.rem_euclid(rhs) }
            }
            pub fn checked_div_euclid(self, rhs: $t) -> Option<$t> {
                if rhs == 0 || (self == $min && rhs == -1) { None } else { Some(self.div_euclid(rhs)) }
            }
            pub fn checked_rem_euclid(self, rhs: $t) -> Option<$t> {
                if rhs == 0 || (self == $min && rhs == -1) { None } else { Some(self.rem_euclid(rhs)) }
            }
            pub fn overflowing_div(self, rhs: $t) -> ($t, bool) {
                if self == $min && rhs == -1 { (self, true) } else { (self / rhs, false) }
            }
            pub fn overflowing_rem(self, rhs: $t) -> ($t, bool) {
                if rhs == -1 { (0, self == $min) } else { (self % rhs, false) }
            }
            pub fn overflowing_abs(self) -> ($t, bool) { (self.wrapping_abs(), self == $min) }
            pub fn saturating_div(self, rhs: $t) -> $t {
                if self == $min && rhs == -1 { $max } else { self / rhs }
            }
            pub fn overflowing_pow(self, exp: u32) -> ($t, bool) {
                match self.checked_pow(exp) {
                    Some(v) => (v, false),
                    None => (self.wrapping_pow(exp), true),
                }
            }
            pub fn checked_ilog2(self) -> Option<u32> {
                if self <= 0 { None } else { Some($bits - 1 - self.leading_zeros()) }
            }
            pub fn ilog2(self) -> u32 { $bits - 1 - self.leading_zeros() }
            pub fn checked_add_unsigned(self, rhs: $ut) -> Option<$t> {
                let r = self + (rhs as $t);
                if r < self || (rhs as $t) < 0 { None } else { Some(r) }
            }
            pub fn wrapping_add_unsigned(self, rhs: $ut) -> $t { self + (rhs as $t) }
            pub fn wrapping_sub_unsigned(self, rhs: $ut) -> $t { self - (rhs as $t) }
            pub fn min_value() -> $t { $min }
            pub fn max_value() -> $t { $max }
            pub fn cast_unsigned(self) -> $ut { self as $ut }
            pub fn saturating_add_unsigned(self, rhs: $ut) -> $t {
                match self.checked_add_unsigned(rhs) {
                    Some(v) => v,
                    None => $max,
                }
            }
            pub fn saturating_sub_unsigned(self, rhs: $ut) -> $t {
                let r = self - (rhs as $t);
                if r > self || (rhs as $t) < 0 { $min } else { r }
            }
            pub fn checked_sub_unsigned(self, rhs: $ut) -> Option<$t> {
                let r = self - (rhs as $t);
                if r > self || (rhs as $t) < 0 { None } else { Some(r) }
            }
        }
    };
}

uint_impl!(u8, i8, 8, 0xff);
uint_impl!(u16, i16, 16, 0xffff);
uint_impl!(u32, i32, 32, 0xffff_ffff);
uint_impl!(u64, i64, 64, 0xffff_ffff_ffff_ffff);
uint_impl!(u128, i128, 128, 0xffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
uint_impl!(usize, isize, 64, 0xffff_ffff_ffff_ffff);
sint_impl!(i8, u8, 8, -0x80, 0x7f);
sint_impl!(i16, u16, 16, -0x8000, 0x7fff);
sint_impl!(i32, u32, 32, -0x8000_0000, 0x7fff_ffff);
sint_impl!(i64, u64, 64, -0x8000_0000_0000_0000, 0x7fff_ffff_ffff_ffff);
sint_impl!(i128, u128, 128, -0x8000_0000_0000_0000_0000_0000_0000_0000, 0x7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
sint_impl!(isize, usize, 64, -0x8000_0000_0000_0000, 0x7fff_ffff_ffff_ffff);

impl u8 {
    pub const fn is_ascii(&self) -> bool { *self < 128 }
    pub const fn is_ascii_digit(&self) -> bool { *self >= b'0' && *self <= b'9' }
    pub const fn is_ascii_hexdigit(&self) -> bool {
        (*self >= b'0' && *self <= b'9') || (*self >= b'a' && *self <= b'f') || (*self >= b'A' && *self <= b'F')
    }
    pub const fn is_ascii_alphabetic(&self) -> bool {
        (*self >= b'a' && *self <= b'z') || (*self >= b'A' && *self <= b'Z')
    }
    pub const fn is_ascii_alphanumeric(&self) -> bool { self.is_ascii_alphabetic() || self.is_ascii_digit() }
    pub const fn is_ascii_uppercase(&self) -> bool { *self >= b'A' && *self <= b'Z' }
    pub const fn is_ascii_lowercase(&self) -> bool { *self >= b'a' && *self <= b'z' }
    pub const fn is_ascii_whitespace(&self) -> bool {
        *self == b' ' || *self == b'\t' || *self == b'\n' || *self == 12 || *self == b'\r'
    }
    pub const fn is_ascii_punctuation(&self) -> bool {
        (*self >= 33 && *self <= 47) || (*self >= 58 && *self <= 64) || (*self >= 91 && *self <= 96) || (*self >= 123 && *self <= 126)
    }
    pub const fn is_ascii_graphic(&self) -> bool { *self >= 33 && *self <= 126 }
    pub const fn is_ascii_control(&self) -> bool { *self < 32 || *self == 127 }
    pub const fn to_ascii_uppercase(&self) -> u8 {
        if self.is_ascii_lowercase() { *self - 32 } else { *self }
    }
    pub const fn to_ascii_lowercase(&self) -> u8 {
        if self.is_ascii_uppercase() { *self + 32 } else { *self }
    }
    pub fn make_ascii_uppercase(&mut self) { *self = self.to_ascii_uppercase() }
    pub fn make_ascii_lowercase(&mut self) { *self = self.to_ascii_lowercase() }
    pub fn eq_ignore_ascii_case(&self, other: &u8) -> bool {
        self.to_ascii_lowercase() == other.to_ascii_lowercase()
    }
}

// Lossless conversions.
macro_rules! impl_from {
    ($($small:ty => $($big:ty)*;)*) => { $( $(
        impl From<$small> for $big {
            fn from(x: $small) -> $big { x as $big }
        }
    )* )* };
}
impl_from! {
    u8 => u16 u32 u64 u128 usize i16 i32 i64 i128 isize f32 f64;
    u16 => u32 u64 u128 usize i32 i64 i128 f32 f64;
    u32 => u64 u128 i64 i128 f64;
    u64 => u128 i128;
    i8 => i16 i32 i64 i128 isize f32 f64;
    i16 => i32 i64 i128 isize f32 f64;
    i32 => i64 i128 f64;
    i64 => i128;
    f32 => f64;
    bool => u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize;
    char => u32 u64 u128;
}
impl From<u8> for char {
    fn from(x: u8) -> char {
        x as char
    }
}

// Checked conversions: go through i128/u128 and compare.
macro_rules! impl_try_from {
    ($($src:ty => $($dst:ty)*;)*) => { $( $(
        impl TryFrom<$src> for $dst {
            type Error = TryFromIntError;
            fn try_from(x: $src) -> Result<$dst, TryFromIntError> {
                let r = x as $dst;
                if (r as $src) == x && ((r < 0) == (x < 0)) {
                    Ok(r)
                } else {
                    Err(crate::num::try_from_int_error())
                }
            }
        }
    )* )* };
}
impl_try_from! {
    u8 => i8;
    u16 => u8 i8 i16;
    u32 => u8 u16 i8 i16 i32 usize isize;
    u64 => u8 u16 u32 i8 i16 i32 i64 usize isize;
    u128 => u8 u16 u32 u64 i8 i16 i32 i64 i128 usize isize;
    usize => u8 u16 u32 u64 i8 i16 i32 i64 isize u128 i128;
    i8 => u8 u16 u32 u64 u128 usize;
    i16 => u8 u16 u32 u64 u128 i8 usize;
    i32 => u8 u16 u32 u64 u128 i8 i16 usize isize;
    i64 => u8 u16 u32 u64 u128 i8 i16 i32 usize isize;
    i128 => u8 u16 u32 u64 u128 i8 i16 i32 i64 usize isize;
    isize => u8 u16 u32 u64 u128 i8 i16 i32 i64 usize i128;
}
impl TryFrom<u32> for char {
    type Error = crate::char::CharTryFromError;
    fn try_from(x: u32) -> Result<char, crate::char::CharTryFromError> {
        match crate::char::from_u32(x) {
            Some(c) => Ok(c),
            None => Err(crate::char::CharTryFromError(())),
        }
    }
}
impl TryFrom<char> for u8 {
    type Error = TryFromIntError;
    fn try_from(c: char) -> Result<u8, TryFromIntError> {
        if (c as u32) < 256 { Ok(c as u32 as u8) } else { Err(crate::num::try_from_int_error()) }
    }
}
