use crate::fmt::{Debug, Display, Formatter, LowerExp, Result, UpperExp};

extern "C" {
    fn rcc_f64_shortest(x: f64, buf: *mut u8, exp10: *mut i32, is_f32: i32) -> i32;
    fn rcc_f64_fixed(x: f64, precision: i32, buf: *mut u8, cap: usize) -> i32;
    fn rcc_f64_exp(x: f64, precision: i32, upper: i32, buf: *mut u8, cap: usize) -> i32;
}

struct Buf {
    data: [u8; 400],
    len: usize,
}

impl Buf {
    fn push(&mut self, b: u8) {
        if self.len < 400 {
            self.data[self.len] = b;
            self.len += 1;
        }
    }
    fn push_str(&mut self, s: &[u8]) {
        let mut i = 0;
        while i < s.len() {
            self.push(s[i]);
            i += 1;
        }
    }
    fn as_str(&self) -> &str {
        unsafe { crate::str::from_utf8_unchecked(crate::slice::from_raw_parts(self.data.as_ptr(), self.len)) }
    }
}

fn push_int(out: &mut Buf, mut n: i32) {
    if n < 0 {
        out.push(b'-');
        n = 0 - n;
    }
    let mut tmp = [0u8; 12];
    let mut i = 12;
    loop {
        i -= 1;
        tmp[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out.push_str(&tmp[i..]);
}

/// Format a finite, non-negative float. `mode`: 0 Display, 1 Debug,
/// 2 `{:e}`, 3 `{:E}`.
fn format_abs(x: f64, is_f32: bool, precision: Option<usize>, mode: u8, out: &mut Buf) {
    if mode >= 2 {
        if let Some(p) = precision {
            let n = unsafe { rcc_f64_exp(x, p as i32, (mode == 3) as i32, out.data.as_mut_ptr(), 400) };
            out.len = n as usize;
            return;
        }
    } else if let Some(p) = precision {
        let n = unsafe { rcc_f64_fixed(x, p as i32, out.data.as_mut_ptr(), 400) };
        out.len = n as usize;
        return;
    }
    if x == 0.0 {
        if mode >= 2 {
            out.push_str(if mode == 3 { b"0E0" } else { b"0e0" });
        } else {
            out.push_str(if mode == 1 { b"0.0" } else { b"0" });
        }
        return;
    }
    let mut digits = [0u8; 24];
    let mut exp10: i32 = 0;
    let nd = unsafe { rcc_f64_shortest(x, digits.as_mut_ptr(), &mut exp10, is_f32 as i32) } as usize;
    let sci = mode >= 2 || (mode == 1 && (exp10 < -3 || exp10 > 16));
    if sci {
        out.push(digits[0]);
        if nd > 1 {
            out.push(b'.');
            out.push_str(&digits[1..nd]);
        }
        out.push(if mode == 3 { b'E' } else { b'e' });
        push_int(out, exp10 - 1);
        return;
    }
    if exp10 <= 0 {
        out.push_str(b"0.");
        let mut z = 0;
        while z < 0 - exp10 {
            out.push(b'0');
            z += 1;
        }
        out.push_str(&digits[..nd]);
    } else if (exp10 as usize) >= nd {
        out.push_str(&digits[..nd]);
        let mut z = nd;
        while z < exp10 as usize {
            out.push(b'0');
            z += 1;
        }
        if mode == 1 {
            out.push_str(b".0");
        }
    } else {
        let e = exp10 as usize;
        out.push_str(&digits[..e]);
        out.push(b'.');
        out.push_str(&digits[e..nd]);
    }
}

fn fmt_float(f: &mut Formatter<'_>, x: f64, is_f32: bool, mode: u8) -> Result {
    if x != x {
        return f.pad_integral(true, "", "NaN");
    }
    let neg = x.is_sign_negative();
    let ax = x.abs();
    if ax.is_infinite() {
        return f.pad_integral(!neg, "", "inf");
    }
    let mut out = Buf { data: [0u8; 400], len: 0 };
    format_abs(ax, is_f32, f.spec.precision, mode, &mut out);
    f.pad_integral(!neg, "", out.as_str())
}

impl Display for f64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self, false, 0)
    }
}
impl Debug for f64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self, false, 1)
    }
}
impl LowerExp for f64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self, false, 2)
    }
}
impl UpperExp for f64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self, false, 3)
    }
}
impl Display for f32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self as f64, true, 0)
    }
}
impl Debug for f32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self as f64, true, 1)
    }
}
impl LowerExp for f32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self as f64, true, 2)
    }
}
impl UpperExp for f32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_float(f, *self as f64, true, 3)
    }
}
