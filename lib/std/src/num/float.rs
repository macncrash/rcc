use crate::intrinsics;
use crate::num::FpCategory;

extern "C" {
    fn sqrt(x: f64) -> f64;
    fn floor(x: f64) -> f64;
    fn ceil(x: f64) -> f64;
    fn round(x: f64) -> f64;
    fn trunc(x: f64) -> f64;
    fn pow(x: f64, y: f64) -> f64;
    fn exp(x: f64) -> f64;
    fn exp2(x: f64) -> f64;
    fn log(x: f64) -> f64;
    fn log2(x: f64) -> f64;
    fn log10(x: f64) -> f64;
    fn sin(x: f64) -> f64;
    fn cos(x: f64) -> f64;
    fn tan(x: f64) -> f64;
    fn atan(x: f64) -> f64;
    fn atan2(y: f64, x: f64) -> f64;
    fn cbrt(x: f64) -> f64;
    fn hypot(x: f64, y: f64) -> f64;
    fn fma(x: f64, y: f64, z: f64) -> f64;
    fn rint(x: f64) -> f64;
    fn rintf(x: f32) -> f32;
    fn asin(x: f64) -> f64;
    fn acos(x: f64) -> f64;
    fn sinh(x: f64) -> f64;
    fn cosh(x: f64) -> f64;
    fn tanh(x: f64) -> f64;
    fn expm1(x: f64) -> f64;
    fn log1p(x: f64) -> f64;
    fn sqrtf(x: f32) -> f32;
    fn floorf(x: f32) -> f32;
    fn ceilf(x: f32) -> f32;
    fn roundf(x: f32) -> f32;
    fn truncf(x: f32) -> f32;
    fn powf(x: f32, y: f32) -> f32;
}

macro_rules! float_common {
    ($t:ident, $bits_ty:ident, $nbytes:expr, $sign_mask:expr, $exp_mask:expr, $man_mask:expr) => {
        pub fn to_bits(self) -> $bits_ty { unsafe { intrinsics::transmute(self) } }
        pub fn from_bits(v: $bits_ty) -> $t { unsafe { intrinsics::transmute(v) } }
        pub fn is_nan(self) -> bool { self != self }
        pub fn is_infinite(self) -> bool { (self.to_bits() & !$sign_mask) == $exp_mask }
        pub fn is_finite(self) -> bool { (self.to_bits() & $exp_mask) != $exp_mask }
        pub fn is_sign_negative(self) -> bool { (self.to_bits() & $sign_mask) != 0 }
        pub fn is_sign_positive(self) -> bool { (self.to_bits() & $sign_mask) == 0 }
        pub fn is_subnormal(self) -> bool {
            let b = self.to_bits();
            (b & $exp_mask) == 0 && (b & $man_mask) != 0
        }
        pub fn is_normal(self) -> bool {
            let e = self.to_bits() & $exp_mask;
            e != 0 && e != $exp_mask
        }
        pub fn classify(self) -> FpCategory {
            let b = self.to_bits();
            let e = b & $exp_mask;
            let m = b & $man_mask;
            if e == $exp_mask {
                if m == 0 { FpCategory::Infinite } else { FpCategory::Nan }
            } else if e == 0 {
                if m == 0 { FpCategory::Zero } else { FpCategory::Subnormal }
            } else {
                FpCategory::Normal
            }
        }
        pub fn abs(self) -> $t { <$t>::from_bits(self.to_bits() & !$sign_mask) }
        pub fn signum(self) -> $t {
            if self.is_nan() { self } else if self.is_sign_negative() { -1.0 } else { 1.0 }
        }
        pub fn copysign(self, sign: $t) -> $t {
            <$t>::from_bits((self.to_bits() & !$sign_mask) | (sign.to_bits() & $sign_mask))
        }
        pub fn min(self, other: $t) -> $t {
            if self.is_nan() { other } else if other.is_nan() { self } else if other < self { other } else { self }
        }
        pub fn max(self, other: $t) -> $t {
            if self.is_nan() { other } else if other.is_nan() { self } else if other > self { other } else { self }
        }
        pub fn clamp(self, min: $t, max: $t) -> $t {
            let mut x = self;
            if x < min { x = min; }
            if x > max { x = max; }
            x
        }
        pub fn total_cmp(&self, other: &$t) -> crate::cmp::Ordering {
            let mut a = self.to_bits();
            let mut b = other.to_bits();
            // flip so that the unsigned order matches the IEEE total order
            if a & $sign_mask != 0 { a = !a } else { a = a | $sign_mask }
            if b & $sign_mask != 0 { b = !b } else { b = b | $sign_mask }
            a.cmp(&b)
        }
        pub fn recip(self) -> $t { 1.0 / self }
        pub fn to_degrees(self) -> $t { self * (180.0 / 3.14159265358979323846264338327950288) }
        pub fn to_radians(self) -> $t { self * (3.14159265358979323846264338327950288 / 180.0) }
        pub fn powi(self, n: i32) -> $t {
            let mut base = self;
            let mut e = if n < 0 { (0 - (n as i64)) as u64 } else { n as u64 };
            let mut acc: $t = 1.0;
            while e > 0 {
                if e & 1 == 1 { acc = acc * base; }
                base = base * base;
                e >>= 1;
            }
            if n < 0 { 1.0 / acc } else { acc }
        }
        pub fn to_ne_bytes(self) -> [u8; $nbytes] { self.to_bits().to_ne_bytes() }
        pub fn to_le_bytes(self) -> [u8; $nbytes] { self.to_bits().to_le_bytes() }
        pub fn to_be_bytes(self) -> [u8; $nbytes] { self.to_bits().to_be_bytes() }
        pub fn from_le_bytes(b: [u8; $nbytes]) -> $t { <$t>::from_bits(<$bits_ty>::from_le_bytes(b)) }
        pub fn from_be_bytes(b: [u8; $nbytes]) -> $t { <$t>::from_bits(<$bits_ty>::from_be_bytes(b)) }
        pub fn from_ne_bytes(b: [u8; $nbytes]) -> $t { <$t>::from_bits(<$bits_ty>::from_ne_bytes(b)) }
    };
}

impl f64 {
    pub const INFINITY: f64 = 1.0 / 0.0;
    pub const NEG_INFINITY: f64 = -1.0 / 0.0;
    pub const NAN: f64 = 0.0 / 0.0;
    pub const MAX: f64 = 1.7976931348623157e308;
    pub const MIN: f64 = -1.7976931348623157e308;
    pub const MIN_POSITIVE: f64 = 2.2250738585072014e-308;
    pub const EPSILON: f64 = 2.220446049250313e-16;
    pub const MANTISSA_DIGITS: u32 = 53;
    pub const DIGITS: u32 = 15;
    pub const RADIX: u32 = 2;
    pub const MAX_EXP: i32 = 1024;
    pub const MIN_EXP: i32 = -1021;

    float_common!(f64, u64, 8, 0x8000_0000_0000_0000u64, 0x7ff0_0000_0000_0000u64, 0x000f_ffff_ffff_ffffu64);

    pub fn sqrt(self) -> f64 { unsafe { sqrt(self) } }
    pub fn floor(self) -> f64 { unsafe { floor(self) } }
    pub fn ceil(self) -> f64 { unsafe { ceil(self) } }
    pub fn round(self) -> f64 { unsafe { round(self) } }
    pub fn trunc(self) -> f64 { unsafe { trunc(self) } }
    pub fn fract(self) -> f64 { self - self.trunc() }
    pub fn round_ties_even(self) -> f64 { unsafe { rint(self) } }
    pub fn asin(self) -> f64 { unsafe { asin(self) } }
    pub fn acos(self) -> f64 { unsafe { acos(self) } }
    pub fn sinh(self) -> f64 { unsafe { sinh(self) } }
    pub fn cosh(self) -> f64 { unsafe { cosh(self) } }
    pub fn tanh(self) -> f64 { unsafe { tanh(self) } }
    pub fn exp_m1(self) -> f64 { unsafe { expm1(self) } }
    pub fn ln_1p(self) -> f64 { unsafe { log1p(self) } }
    pub fn div_euclid(self, rhs: f64) -> f64 {
        let q = (self / rhs).trunc();
        if self % rhs < 0.0 {
            if rhs > 0.0 { q - 1.0 } else { q + 1.0 }
        } else {
            q
        }
    }
    pub fn powf(self, n: f64) -> f64 { unsafe { pow(self, n) } }
    pub fn exp(self) -> f64 { unsafe { exp(self) } }
    pub fn exp2(self) -> f64 { unsafe { exp2(self) } }
    pub fn ln(self) -> f64 { unsafe { log(self) } }
    pub fn log2(self) -> f64 { unsafe { log2(self) } }
    pub fn log10(self) -> f64 { unsafe { log10(self) } }
    pub fn log(self, base: f64) -> f64 { self.ln() / base.ln() }
    pub fn sin(self) -> f64 { unsafe { sin(self) } }
    pub fn cos(self) -> f64 { unsafe { cos(self) } }
    pub fn tan(self) -> f64 { unsafe { tan(self) } }
    pub fn atan(self) -> f64 { unsafe { atan(self) } }
    pub fn atan2(self, other: f64) -> f64 { unsafe { atan2(self, other) } }
    pub fn cbrt(self) -> f64 { unsafe { cbrt(self) } }
    pub fn hypot(self, other: f64) -> f64 { unsafe { hypot(self, other) } }
    pub fn mul_add(self, a: f64, b: f64) -> f64 { unsafe { fma(self, a, b) } }
    pub fn rem_euclid(self, rhs: f64) -> f64 {
        let r = self % rhs;
        if r < 0.0 { r + rhs.abs() } else { r }
    }
}

impl f32 {
    pub const INFINITY: f32 = 1.0 / 0.0;
    pub const NEG_INFINITY: f32 = -1.0 / 0.0;
    pub const NAN: f32 = 0.0 / 0.0;
    pub const MAX: f32 = 3.40282347e38;
    pub const MIN: f32 = -3.40282347e38;
    pub const MIN_POSITIVE: f32 = 1.17549435e-38;
    pub const EPSILON: f32 = 1.1920929e-7;
    pub const MANTISSA_DIGITS: u32 = 24;
    pub const DIGITS: u32 = 6;
    pub const RADIX: u32 = 2;

    float_common!(f32, u32, 4, 0x8000_0000u32, 0x7f80_0000u32, 0x007f_ffffu32);

    pub fn sqrt(self) -> f32 { unsafe { sqrtf(self) } }
    pub fn floor(self) -> f32 { unsafe { floorf(self) } }
    pub fn ceil(self) -> f32 { unsafe { ceilf(self) } }
    pub fn round(self) -> f32 { unsafe { roundf(self) } }
    pub fn trunc(self) -> f32 { unsafe { truncf(self) } }
    pub fn fract(self) -> f32 { self - self.trunc() }
    pub fn round_ties_even(self) -> f32 { unsafe { rintf(self) } }
    pub fn tan(self) -> f32 { (self as f64).tan() as f32 }
    pub fn mul_add(self, a: f32, b: f32) -> f32 { ((self as f64) * (a as f64) + (b as f64)) as f32 }
    pub fn hypot(self, other: f32) -> f32 { (self as f64).hypot(other as f64) as f32 }
    pub fn cbrt(self) -> f32 { (self as f64).cbrt() as f32 }
    pub fn atan2(self, other: f32) -> f32 { (self as f64).atan2(other as f64) as f32 }
    pub fn rem_euclid(self, rhs: f32) -> f32 {
        let r = self % rhs;
        if r < 0.0 { r + rhs.abs() } else { r }
    }
    pub fn powf(self, n: f32) -> f32 { unsafe { powf(self, n) } }
    pub fn ln(self) -> f32 { (self as f64).ln() as f32 }
    pub fn log2(self) -> f32 { (self as f64).log2() as f32 }
    pub fn log10(self) -> f32 { (self as f64).log10() as f32 }
    pub fn exp(self) -> f32 { (self as f64).exp() as f32 }
    pub fn sin(self) -> f32 { (self as f64).sin() as f32 }
    pub fn cos(self) -> f32 { (self as f64).cos() as f32 }
}

pub mod consts_f64 {
    pub const PI: f64 = 3.14159265358979323846264338327950288;
    pub const TAU: f64 = 6.28318530717958647692528676655900577;
    pub const E: f64 = 2.71828182845904523536028747135266250;
    pub const SQRT_2: f64 = 1.41421356237309504880168872420969808;
    pub const LN_2: f64 = 0.693147180559945309417232121458176568;
    pub const LN_10: f64 = 2.30258509299404568401799145468436421;
    pub const LOG2_E: f64 = 1.44269504088896340735992468100189214;
    pub const LOG10_E: f64 = 0.434294481903251827651128918916605082;
    pub const FRAC_PI_2: f64 = 1.57079632679489661923132169163975144;
    pub const FRAC_PI_4: f64 = 0.785398163397448309615660845819875721;
    pub const FRAC_1_PI: f64 = 0.318309886183790671537767526745028724;
    pub const FRAC_1_SQRT_2: f64 = 0.707106781186547524400844362104849039;
}
pub mod consts_f32 {
    pub const PI: f32 = 3.14159265358979323846264338327950288;
    pub const TAU: f32 = 6.28318530717958647692528676655900577;
    pub const E: f32 = 2.71828182845904523536028747135266250;
    pub const SQRT_2: f32 = 1.41421356237309504880168872420969808;
    pub const LN_2: f32 = 0.693147180559945309417232121458176568;
    pub const FRAC_PI_2: f32 = 1.57079632679489661923132169163975144;
}
