use crate::cmp::Ordering;
use crate::fmt;
use crate::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use crate::slice::SliceIndex;
use crate::string::String;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Utf8Error {
    valid_up_to: usize,
    error_len: Option<u8>,
}
impl Utf8Error {
    pub fn valid_up_to(&self) -> usize {
        self.valid_up_to
    }
    /// `None`: the input ended inside a sequence; `Some(n)`: the n bytes at
    /// `valid_up_to` are invalid.
    pub fn error_len(&self) -> Option<usize> {
        match self.error_len {
            Some(n) => Some(n as usize),
            None => None,
        }
    }
}
impl fmt::Display for Utf8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.error_len {
            Some(n) => write!(f, "invalid utf-8 sequence of {} bytes from index {}", n, self.valid_up_to),
            None => write!(f, "incomplete utf-8 byte sequence from index {}", self.valid_up_to),
        }
    }
}

pub(crate) fn validate_utf8(v: &[u8]) -> Result<(), Utf8Error> {
    let n = v.len();
    let mut i = 0;
    while i < n {
        let b = v[i];
        if b < 0x80 {
            i += 1;
            continue;
        }
        // (number of continuation bytes, allowed range of the first one)
        let (need, lo, hi) = if b >= 0xC2 && b < 0xE0 {
            (1, 0x80u8, 0xBFu8)
        } else if b == 0xE0 {
            (2, 0xA0, 0xBF)
        } else if b == 0xED {
            (2, 0x80, 0x9F)
        } else if b >= 0xE1 && b < 0xF0 {
            (2, 0x80, 0xBF)
        } else if b == 0xF0 {
            (3, 0x90, 0xBF)
        } else if b == 0xF4 {
            (3, 0x80, 0x8F)
        } else if b >= 0xF1 && b < 0xF4 {
            (3, 0x80, 0xBF)
        } else {
            return Err(Utf8Error { valid_up_to: i, error_len: Some(1) });
        };
        let mut k = 1;
        while k <= need {
            if i + k >= n {
                return Err(Utf8Error { valid_up_to: i, error_len: None });
            }
            let c = v[i + k];
            let ok = if k == 1 { c >= lo && c <= hi } else { (c & 0xC0) == 0x80 };
            if !ok {
                return Err(Utf8Error { valid_up_to: i, error_len: Some(k as u8) });
            }
            k += 1;
        }
        i += need + 1;
    }
    Ok(())
}

impl PartialEq for str {
    fn eq(&self, other: &str) -> bool {
        let a = self.as_bytes();
        let b = other.as_bytes();
        if a.len() != b.len() {
            return false;
        }
        let mut i = 0;
        while i < a.len() {
            if a[i] != b[i] {
                return false;
            }
            i += 1;
        }
        true
    }
}
impl Eq for str {}
impl PartialOrd for str {
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for str {
    fn cmp(&self, other: &str) -> Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl SliceIndex<str> for Range<usize> {
    type Output = str;
    fn get(self, s: &str) -> Option<&str> {
        if self.start <= self.end && s.is_char_boundary(self.start) && s.is_char_boundary(self.end) {
            Some(unsafe { s.get_unchecked(self.start, self.end) })
        } else {
            None
        }
    }
    fn get_mut(self, s: &mut str) -> Option<&mut str> {
        if self.start <= self.end && s.is_char_boundary(self.start) && s.is_char_boundary(self.end) {
            Some(unsafe { crate::str::from_utf8_unchecked_mut(&mut s.as_bytes_mut()[self.start..self.end]) })
        } else {
            None
        }
    }
    fn index(self, s: &str) -> &str {
        s.slice(self.start, self.end)
    }
    fn index_mut(self, s: &mut str) -> &mut str {
        s.slice(self.start, self.end);
        unsafe { crate::str::from_utf8_unchecked_mut(&mut s.as_bytes_mut()[self.start..self.end]) }
    }
}

macro_rules! str_index_via_range {
    ($t:ty, |$r:ident, $s:ident| $range:expr) => {
        impl SliceIndex<str> for $t {
            type Output = str;
            fn get(self, $s: &str) -> Option<&str> {
                let $r = self;
                ($range).get($s)
            }
            fn get_mut(self, $s: &mut str) -> Option<&mut str> {
                let $r = self;
                let range = $range;
                range.get_mut($s)
            }
            fn index(self, $s: &str) -> &str {
                let $r = self;
                ($range).index($s)
            }
            fn index_mut(self, $s: &mut str) -> &mut str {
                let $r = self;
                let range = $range;
                range.index_mut($s)
            }
        }
    };
}
str_index_via_range!(RangeTo<usize>, |r, s| 0..r.end);
str_index_via_range!(RangeFrom<usize>, |r, s| r.start..s.len());
str_index_via_range!(RangeFull, |_r, s| 0..s.len());
str_index_via_range!(RangeInclusive<usize>, |r, s| r.start..r.end + 1);
str_index_via_range!(RangeToInclusive<usize>, |r, s| 0..r.end + 1);

impl<I: SliceIndex<str>> crate::ops::Index<I> for str {
    type Output = I::Output;
    fn index(&self, index: I) -> &I::Output {
        index.index(self)
    }
}
impl<I: SliceIndex<str>> crate::ops::IndexMut<I> for str {
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        index.index_mut(self)
    }
}

impl AsRef<str> for str {
    fn as_ref(&self) -> &str {
        self
    }
}
impl AsRef<[u8]> for str {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl crate::borrow::ToOwned for str {
    type Owned = String;
    fn to_owned(&self) -> String {
        String::from(self)
    }
}

pub trait FromStr: Sized {
    type Err;
    fn from_str(s: &str) -> Result<Self, Self::Err>;
}

macro_rules! from_str_int {
    ($($t:ty)*) => { $(
        impl FromStr for $t {
            type Err = crate::num::ParseIntError;
            fn from_str(s: &str) -> Result<$t, crate::num::ParseIntError> {
                <$t>::from_str_radix(s, 10)
            }
        }
    )* };
}
from_str_int! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize }

extern "C" {
    fn rcc_strtod(s: *const u8, len: usize, ok: *mut i32) -> f64;
    fn rcc_strtof(s: *const u8, len: usize, ok: *mut i32) -> f32;
}

/// Rust's float grammar is stricter than strtod's: no leading whitespace,
/// no hex floats, only `inf`/`infinity`/`nan` (any case) as words.
fn float_syntax_ok(b: &[u8]) -> bool {
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let rest = &b[i..];
    if rest.eq_ignore_ascii_case(b"inf") || rest.eq_ignore_ascii_case(b"infinity") || rest.eq_ignore_ascii_case(b"nan") {
        return true;
    }
    let mut digits = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return false;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let mut ed = 0;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            ed += 1;
        }
        if ed == 0 {
            return false;
        }
    }
    i == b.len()
}

impl FromStr for f64 {
    type Err = crate::num::ParseFloatError;
    fn from_str(s: &str) -> Result<f64, crate::num::ParseFloatError> {
        if s.len() == 0 || !float_syntax_ok(s.as_bytes()) {
            return Err(crate::num::float_error(s.len() == 0));
        }
        let mut ok: i32 = 0;
        let v = unsafe { rcc_strtod(s.as_ptr(), s.len(), &mut ok) };
        if ok != 0 { Ok(v) } else { Err(crate::num::float_error(false)) }
    }
}
impl FromStr for f32 {
    type Err = crate::num::ParseFloatError;
    fn from_str(s: &str) -> Result<f32, crate::num::ParseFloatError> {
        if s.len() == 0 || !float_syntax_ok(s.as_bytes()) {
            return Err(crate::num::float_error(s.len() == 0));
        }
        let mut ok: i32 = 0;
        let v = unsafe { rcc_strtof(s.as_ptr(), s.len(), &mut ok) };
        if ok != 0 { Ok(v) } else { Err(crate::num::float_error(false)) }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParseBoolError(());
impl fmt::Display for ParseBoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("provided string was not `true` or `false`")
    }
}
impl FromStr for bool {
    type Err = ParseBoolError;
    fn from_str(s: &str) -> Result<bool, ParseBoolError> {
        if s == "true" {
            Ok(true)
        } else if s == "false" {
            Ok(false)
        } else {
            Err(ParseBoolError(()))
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParseCharError(());
impl fmt::Display for ParseCharError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("too many characters in string")
    }
}
impl FromStr for char {
    type Err = ParseCharError;
    fn from_str(s: &str) -> Result<char, ParseCharError> {
        let mut it = s.chars();
        match (it.next(), it.next()) {
            (Some(c), None) => Ok(c),
            _ => Err(ParseCharError(())),
        }
    }
}
impl FromStr for String {
    type Err = crate::convert::Infallible;
    fn from_str(s: &str) -> Result<String, crate::convert::Infallible> {
        Ok(String::from(s))
    }
}
