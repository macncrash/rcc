use crate::fmt::{Binary, Debug, Display, Formatter, LowerExp, LowerHex, Octal, Result, UpperExp, UpperHex};

/// Write the digits of `n` in `radix` at the end of `buf`; returns the start
/// index.
fn digits_u64(mut n: u64, radix: u64, upper: bool, buf: &mut [u8; 130]) -> usize {
    let mut i = 130;
    loop {
        i -= 1;
        let d = (n % radix) as u8;
        buf[i] = if d < 10 { b'0' + d } else if upper { b'A' + d - 10 } else { b'a' + d - 10 };
        n /= radix;
        if n == 0 {
            break;
        }
    }
    i
}

fn digits_u128(mut n: u128, radix: u128, upper: bool, buf: &mut [u8; 130]) -> usize {
    if n <= 0xffff_ffff_ffff_ffff {
        return digits_u64(n as u64, radix as u64, upper, buf);
    }
    let mut i = 130;
    loop {
        i -= 1;
        let d = (n % radix) as u8;
        buf[i] = if d < 10 { b'0' + d } else if upper { b'A' + d - 10 } else { b'a' + d - 10 };
        n /= radix;
        if n == 0 {
            break;
        }
    }
    i
}

fn fmt_radix(f: &mut Formatter<'_>, n: u128, nonneg: bool, radix: u128, upper: bool, prefix: &str) -> Result {
    let mut buf = [0u8; 130];
    let i = digits_u128(n, radix, upper, &mut buf);
    let s = unsafe { crate::str::from_utf8_unchecked(&buf[i..]) };
    f.pad_integral(nonneg, prefix, s)
}

macro_rules! int_fmt {
    ($($t:ty, $ut:ty, $signed:expr;)*) => { $(
        impl Display for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                let v = *self;
                if $signed && (v as i128) < 0 {
                    fmt_radix(f, (0u128).wrapping_sub(v as i128 as u128), false, 10, false, "")
                } else {
                    fmt_radix(f, v as u128, true, 10, false, "")
                }
            }
        }
        impl Debug for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                if f.debug_hex == 1 {
                    LowerHex::fmt(self, f)
                } else if f.debug_hex == 2 {
                    UpperHex::fmt(self, f)
                } else {
                    Display::fmt(self, f)
                }
            }
        }
        impl LowerHex for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                fmt_radix(f, (*self as $ut) as u128, true, 16, false, "0x")
            }
        }
        impl UpperHex for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                fmt_radix(f, (*self as $ut) as u128, true, 16, true, "0x")
            }
        }
        impl Octal for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                fmt_radix(f, (*self as $ut) as u128, true, 8, false, "0o")
            }
        }
        impl Binary for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                fmt_radix(f, (*self as $ut) as u128, true, 2, false, "0b")
            }
        }
        impl LowerExp for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                LowerExp::fmt(&(*self as f64), f)
            }
        }
        impl UpperExp for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                UpperExp::fmt(&(*self as f64), f)
            }
        }
    )* };
}
int_fmt! {
    u8, u8, false; u16, u16, false; u32, u32, false; u64, u64, false; u128, u128, false; usize, usize, false;
    i8, u8, true; i16, u16, true; i32, u32, true; i64, u64, true; i128, u128, true; isize, usize, true;
}
