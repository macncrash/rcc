//! String slices.

mod iter;
mod pattern;
mod traits;

pub use self::iter::{Bytes, CharIndices, Chars, Lines, MatchIndices, Matches, RSplit, RSplitN, Split, SplitAsciiWhitespace, SplitInclusive, SplitN, SplitTerminator, SplitWhitespace};
pub use self::pattern::Pattern;
pub use self::traits::{FromStr, ParseBoolError, ParseCharError, Utf8Error};

use crate::intrinsics;
use crate::string::String;

pub fn from_utf8(v: &[u8]) -> Result<&str, Utf8Error> {
    match traits::validate_utf8(v) {
        Ok(()) => Ok(unsafe { from_utf8_unchecked(v) }),
        Err(e) => Err(e),
    }
}
pub fn from_utf8_mut(v: &mut [u8]) -> Result<&mut str, Utf8Error> {
    match traits::validate_utf8(v) {
        Ok(()) => Ok(unsafe { from_utf8_unchecked_mut(v) }),
        Err(e) => Err(e),
    }
}
pub const unsafe fn from_utf8_unchecked(v: &[u8]) -> &str {
    intrinsics::transmute(v)
}
pub unsafe fn from_utf8_unchecked_mut(v: &mut [u8]) -> &mut str {
    intrinsics::transmute(v)
}

fn boundary_fail(s: &str, index: usize) -> ! {
    if index > s.len() {
        panic!("byte index {} is out of bounds of string of length {}", index, s.len())
    }
    panic!("byte index {} is not a char boundary", index)
}

/// Decode the char starting at byte `i`; returns (char, byte length).
pub(crate) fn decode_at(b: &[u8], i: usize) -> (char, usize) {
    let x = b[i] as u32;
    if x < 0x80 {
        return (unsafe { crate::char::from_u32_unchecked(x) }, 1);
    }
    if x < 0xE0 {
        let c = ((x & 0x1F) << 6) | (b[i + 1] as u32 & 0x3F);
        return (unsafe { crate::char::from_u32_unchecked(c) }, 2);
    }
    if x < 0xF0 {
        let c = ((x & 0x0F) << 12) | ((b[i + 1] as u32 & 0x3F) << 6) | (b[i + 2] as u32 & 0x3F);
        return (unsafe { crate::char::from_u32_unchecked(c) }, 3);
    }
    let c = ((x & 0x07) << 18) | ((b[i + 1] as u32 & 0x3F) << 12) | ((b[i + 2] as u32 & 0x3F) << 6) | (b[i + 3] as u32 & 0x3F);
    (unsafe { crate::char::from_u32_unchecked(c) }, 4)
}

/// Decode the char ending just before byte `end`; returns (char, start).
pub(crate) fn decode_before(b: &[u8], end: usize) -> (char, usize) {
    let mut i = end - 1;
    while i > 0 && (b[i] & 0xC0) == 0x80 {
        i -= 1;
    }
    (decode_at(b, i).0, i)
}

impl str {
    pub const fn len(&self) -> usize {
        self.as_bytes().len()
    }
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { intrinsics::transmute(self) }
    }
    pub unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        intrinsics::transmute(self)
    }
    pub const fn as_ptr(&self) -> *const u8 {
        self.as_bytes().as_ptr()
    }
    pub fn is_char_boundary(&self, index: usize) -> bool {
        if index == 0 || index == self.len() {
            return true;
        }
        if index > self.len() {
            return false;
        }
        (self.as_bytes()[index] & 0xC0) != 0x80
    }
    /// `&self[start..end]` with boundary checks.
    pub(crate) fn slice(&self, start: usize, end: usize) -> &str {
        if start > end {
            panic!("begin <= end ({} <= {}) when slicing string", start, end);
        }
        if !self.is_char_boundary(start) {
            boundary_fail(self, start);
        }
        if !self.is_char_boundary(end) {
            boundary_fail(self, end);
        }
        unsafe { self.get_unchecked(start, end) }
    }
    pub(crate) unsafe fn get_unchecked(&self, start: usize, end: usize) -> &str {
        from_utf8_unchecked(crate::slice::from_raw_parts(self.as_ptr().add(start), end - start))
    }
    pub fn get<I: crate::slice::SliceIndex<str>>(&self, i: I) -> Option<&I::Output> {
        i.get(self)
    }
    pub fn chars(&self) -> Chars<'_> {
        iter::chars_new(self)
    }
    pub fn char_indices(&self) -> CharIndices<'_> {
        iter::char_indices_new(self)
    }
    pub fn bytes(&self) -> Bytes<'_> {
        iter::bytes_new(self)
    }
    pub fn lines(&self) -> Lines<'_> {
        iter::lines_new(self)
    }
    pub fn split_whitespace(&self) -> SplitWhitespace<'_> {
        iter::split_whitespace_new(self)
    }
    pub fn split_ascii_whitespace(&self) -> SplitAsciiWhitespace<'_> {
        iter::split_ascii_whitespace_new(self)
    }
    pub fn split_at(&self, mid: usize) -> (&str, &str) {
        (self.slice(0, mid), self.slice(mid, self.len()))
    }
    pub fn split_at_checked(&self, mid: usize) -> Option<(&str, &str)> {
        if self.is_char_boundary(mid) { Some(self.split_at(mid)) } else { None }
    }
    pub fn contains<P: Pattern>(&self, mut pat: P) -> bool {
        pat.find_in(self, 0).is_some()
    }
    pub fn starts_with<P: Pattern>(&self, mut pat: P) -> bool {
        pat.prefix_len(self).is_some()
    }
    pub fn ends_with<P: Pattern>(&self, mut pat: P) -> bool {
        pat.suffix_len(self).is_some()
    }
    pub fn find<P: Pattern>(&self, mut pat: P) -> Option<usize> {
        match pat.find_in(self, 0) {
            Some((a, _)) => Some(a),
            None => None,
        }
    }
    pub fn rfind<P: Pattern>(&self, mut pat: P) -> Option<usize> {
        match pat.rfind_in(self, self.len()) {
            Some((a, _)) => Some(a),
            None => None,
        }
    }
    pub fn split<P: Pattern>(&self, pat: P) -> Split<'_, P> {
        iter::split_new(self, pat)
    }
    pub fn rsplit<P: Pattern>(&self, pat: P) -> RSplit<'_, P> {
        iter::rsplit_new(self, pat)
    }
    pub fn splitn<P: Pattern>(&self, n: usize, pat: P) -> SplitN<'_, P> {
        iter::splitn_new(self, n, pat)
    }
    pub fn rsplitn<P: Pattern>(&self, n: usize, pat: P) -> RSplitN<'_, P> {
        iter::rsplitn_new(self, n, pat)
    }
    pub fn split_terminator<P: Pattern>(&self, pat: P) -> SplitTerminator<'_, P> {
        iter::split_terminator_new(self, pat)
    }
    pub fn split_inclusive<P: Pattern>(&self, pat: P) -> SplitInclusive<'_, P> {
        iter::split_inclusive_new(self, pat)
    }
    pub fn matches<P: Pattern>(&self, pat: P) -> Matches<'_, P> {
        iter::matches_new(self, pat)
    }
    pub fn match_indices<P: Pattern>(&self, pat: P) -> MatchIndices<'_, P> {
        iter::match_indices_new(self, pat)
    }
    pub fn split_once<P: Pattern>(&self, mut delimiter: P) -> Option<(&str, &str)> {
        match delimiter.find_in(self, 0) {
            Some((a, b)) => unsafe { Some((self.get_unchecked(0, a), self.get_unchecked(b, self.len()))) },
            None => None,
        }
    }
    pub fn rsplit_once<P: Pattern>(&self, mut delimiter: P) -> Option<(&str, &str)> {
        match delimiter.rfind_in(self, self.len()) {
            Some((a, b)) => unsafe { Some((self.get_unchecked(0, a), self.get_unchecked(b, self.len()))) },
            None => None,
        }
    }
    pub fn trim(&self) -> &str {
        self.trim_start().trim_end()
    }
    pub fn trim_start(&self) -> &str {
        let b = self.as_bytes();
        let mut i = 0;
        while i < b.len() {
            let (c, n) = decode_at(b, i);
            if !c.is_whitespace() {
                break;
            }
            i += n;
        }
        unsafe { self.get_unchecked(i, b.len()) }
    }
    pub fn trim_end(&self) -> &str {
        let b = self.as_bytes();
        let mut end = b.len();
        while end > 0 {
            let (c, start) = decode_before(b, end);
            if !c.is_whitespace() {
                break;
            }
            end = start;
        }
        unsafe { self.get_unchecked(0, end) }
    }
    pub fn trim_left(&self) -> &str {
        self.trim_start()
    }
    pub fn trim_right(&self) -> &str {
        self.trim_end()
    }
    pub fn trim_ascii(&self) -> &str {
        let b = self.as_bytes();
        let mut a = 0;
        let mut e = b.len();
        while a < e && b[a].is_ascii_whitespace() {
            a += 1;
        }
        while e > a && b[e - 1].is_ascii_whitespace() {
            e -= 1;
        }
        unsafe { self.get_unchecked(a, e) }
    }
    pub fn trim_start_matches<P: Pattern>(&self, mut pat: P) -> &str {
        let mut s = self;
        loop {
            match pat.prefix_len(s) {
                Some(n) if n > 0 => s = unsafe { s.get_unchecked(n, s.len()) },
                _ => return s,
            }
        }
    }
    pub fn trim_end_matches<P: Pattern>(&self, mut pat: P) -> &str {
        let mut s = self;
        loop {
            match pat.suffix_len(s) {
                Some(n) if n > 0 => s = unsafe { s.get_unchecked(0, s.len() - n) },
                _ => return s,
            }
        }
    }
    pub fn trim_matches<P: Pattern + Clone>(&self, pat: P) -> &str {
        self.trim_start_matches(pat.clone()).trim_end_matches(pat)
    }
    pub fn strip_prefix<P: Pattern>(&self, mut prefix: P) -> Option<&str> {
        match prefix.prefix_len(self) {
            Some(n) => Some(unsafe { self.get_unchecked(n, self.len()) }),
            None => None,
        }
    }
    pub fn strip_suffix<P: Pattern>(&self, mut suffix: P) -> Option<&str> {
        match suffix.suffix_len(self) {
            Some(n) => Some(unsafe { self.get_unchecked(0, self.len() - n) }),
            None => None,
        }
    }
    pub fn parse<F: FromStr>(&self) -> Result<F, F::Err> {
        FromStr::from_str(self)
    }
    pub fn is_ascii(&self) -> bool {
        self.as_bytes().is_ascii()
    }
    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {
        self.as_bytes().eq_ignore_ascii_case(other.as_bytes())
    }
    pub fn to_ascii_lowercase(&self) -> String {
        unsafe { String::from_utf8_unchecked(self.as_bytes().to_ascii_lowercase()) }
    }
    pub fn to_ascii_uppercase(&self) -> String {
        unsafe { String::from_utf8_unchecked(self.as_bytes().to_ascii_uppercase()) }
    }
    pub fn make_ascii_lowercase(&mut self) {
        unsafe { self.as_bytes_mut().make_ascii_lowercase() }
    }
    pub fn make_ascii_uppercase(&mut self) {
        unsafe { self.as_bytes_mut().make_ascii_uppercase() }
    }
    pub fn to_lowercase(&self) -> String {
        let mut s = String::with_capacity(self.len());
        for c in self.chars() {
            s.push(c.to_lower_simple());
        }
        s
    }
    pub fn to_uppercase(&self) -> String {
        let mut s = String::with_capacity(self.len());
        for c in self.chars() {
            for u in c.to_uppercase() {
                s.push(u);
            }
        }
        s
    }
    pub fn repeat(&self, n: usize) -> String {
        let mut s = String::with_capacity(self.len() * n);
        let mut i = 0;
        while i < n {
            s.push_str(self);
            i += 1;
        }
        s
    }
    pub fn replace<P: Pattern>(&self, mut from: P, to: &str) -> String {
        let mut out = String::with_capacity(self.len());
        let mut last = 0;
        let mut pos = 0;
        while let Some((a, b)) = from.find_in(self, pos) {
            out.push_str(unsafe { self.get_unchecked(last, a) });
            out.push_str(to);
            last = b;
            if b > a {
                pos = b;
            } else {
                // empty match: step over one char
                if a >= self.len() {
                    break;
                }
                pos = a + decode_at(self.as_bytes(), a).1;
                out.push_str(unsafe { self.get_unchecked(a, pos) });
                last = pos;
            }
        }
        out.push_str(unsafe { self.get_unchecked(last, self.len()) });
        out
    }
    pub fn replacen<P: Pattern>(&self, mut from: P, to: &str, count: usize) -> String {
        let mut out = String::with_capacity(self.len());
        let mut last = 0;
        let mut n = 0;
        while n < count {
            match from.find_in(self, last) {
                Some((a, b)) if b > a => {
                    out.push_str(unsafe { self.get_unchecked(last, a) });
                    out.push_str(to);
                    last = b;
                }
                _ => break,
            }
            n += 1;
        }
        out.push_str(unsafe { self.get_unchecked(last, self.len()) });
        out
    }
    pub fn into_boxed_bytes(self: crate::boxed::Box<str>) -> crate::boxed::Box<[u8]> {
        unsafe { intrinsics::transmute(self) }
    }
    pub fn into_string(self: crate::boxed::Box<str>) -> String {
        let bytes: crate::boxed::Box<[u8]> = unsafe { intrinsics::transmute(self) };
        unsafe { String::from_utf8_unchecked(bytes.into_vec()) }
    }
    pub fn escape_debug(&self) -> String {
        let mut s = String::new();
        let mut first = true;
        for c in self.chars() {
            // combining marks are only escaped at the start of a string
            if !first && (c as u32) >= 0x300 && (c as u32) <= 0x36F {
                s.push(c);
            } else {
                for e in c.escape_debug() {
                    s.push(e);
                }
            }
            first = false;
        }
        s
    }
    pub fn escape_default(&self) -> String {
        let mut s = String::new();
        for c in self.chars() {
            for e in c.escape_default() {
                s.push(e);
            }
        }
        s
    }
}
