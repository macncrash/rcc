//! `String`: a `Vec<u8>` holding UTF-8.

use crate::borrow::Cow;
use crate::hash::Hash;
use crate::cmp::Ordering;
use crate::fmt;
use crate::ops::{Deref, DerefMut, RangeBounds};
use crate::str;
use crate::vec::Vec;

#[lang = "String"]
pub struct String {
    vec: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FromUtf8Error {
    bytes: Vec<u8>,
    error: str::Utf8Error,
}
impl FromUtf8Error {
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn utf8_error(&self) -> str::Utf8Error {
        self.error
    }
}
impl fmt::Display for FromUtf8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}

impl String {
    pub const fn new() -> String {
        String { vec: Vec::new() }
    }
    pub fn with_capacity(capacity: usize) -> String {
        String { vec: Vec::with_capacity(capacity) }
    }
    pub fn from_utf8(vec: Vec<u8>) -> Result<String, FromUtf8Error> {
        match str::from_utf8(&vec) {
            Ok(_) => Ok(String { vec }),
            Err(e) => Err(FromUtf8Error { bytes: vec, error: e }),
        }
    }
    pub unsafe fn from_utf8_unchecked(bytes: Vec<u8>) -> String {
        String { vec: bytes }
    }
    pub fn from_utf8_lossy(v: &[u8]) -> Cow<'_, str> {
        match str::from_utf8(v) {
            Ok(s) => Cow::Borrowed(s),
            Err(_) => {
                let mut out = String::with_capacity(v.len());
                let mut rest = v;
                loop {
                    match str::from_utf8(rest) {
                        Ok(s) => {
                            out.push_str(s);
                            break;
                        }
                        Err(e) => {
                            let n = e.valid_up_to();
                            out.push_str(unsafe { str::from_utf8_unchecked(&rest[..n]) });
                            out.push('\u{FFFD}');
                            // skip the invalid sequence (one U+FFFD per maximal invalid
                            // prefix, as std); an incomplete sequence ends the input
                            let k = match e.error_len() {
                                Some(len) => n + len,
                                None => rest.len(),
                            };
                            rest = &rest[k..];
                        }
                    }
                }
                Cow::Owned(out)
            }
        }
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.vec
    }
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.vec.as_slice()) }
    }
    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe { str::from_utf8_unchecked_mut(self.vec.as_mut_slice()) }
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.vec.as_slice()
    }
    pub unsafe fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.vec
    }
    pub fn push_str(&mut self, s: &str) {
        self.vec.extend_from_slice(s.as_bytes())
    }
    pub fn push(&mut self, c: char) {
        if (c as u32) < 128 {
            self.vec.push(c as u8);
        } else {
            let mut buf = [0u8; 4];
            let s: &str = c.encode_utf8(&mut buf);
            self.vec.extend_from_slice(s.as_bytes());
        }
    }
    pub fn pop(&mut self) -> Option<char> {
        let n = self.vec.len();
        if n == 0 {
            return None;
        }
        let (c, start) = str::decode_before(self.vec.as_slice(), n);
        self.vec.truncate(start);
        Some(c)
    }
    pub fn len(&self) -> usize {
        self.vec.len()
    }
    pub fn is_empty(&self) -> bool {
        self.vec.len() == 0
    }
    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }
    pub fn reserve(&mut self, additional: usize) {
        self.vec.reserve(additional)
    }
    pub fn reserve_exact(&mut self, additional: usize) {
        self.vec.reserve_exact(additional)
    }
    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit()
    }
    pub fn clear(&mut self) {
        self.vec.clear()
    }
    pub fn truncate(&mut self, new_len: usize) {
        if new_len <= self.len() {
            if !self.as_str().is_char_boundary(new_len) {
                panic!("assertion failed: self.is_char_boundary(new_len)");
            }
            self.vec.truncate(new_len)
        }
    }
    pub fn insert(&mut self, idx: usize, ch: char) {
        let mut buf = [0u8; 4];
        let s: &str = ch.encode_utf8(&mut buf);
        self.insert_str(idx, s)
    }
    pub fn insert_str(&mut self, idx: usize, string: &str) {
        if !self.as_str().is_char_boundary(idx) {
            panic!("assertion failed: self.is_char_boundary(idx)");
        }
        let mut tail = self.vec.split_off(idx);
        self.vec.extend_from_slice(string.as_bytes());
        self.vec.append(&mut tail);
    }
    pub fn remove(&mut self, idx: usize) -> char {
        if !self.as_str().is_char_boundary(idx) || idx >= self.len() {
            panic!("cannot remove a char from the end of a string");
        }
        let (c, n) = str::decode_at(self.vec.as_slice(), idx);
        let mut k = 0;
        while k < n {
            self.vec.remove(idx);
            k += 1;
        }
        c
    }
    pub fn retain<F: FnMut(char) -> bool>(&mut self, mut f: F) {
        let mut out = String::with_capacity(self.len());
        for c in self.as_str().chars() {
            if f(c) {
                out.push(c);
            }
        }
        *self = out;
    }
    pub fn split_off(&mut self, at: usize) -> String {
        if !self.as_str().is_char_boundary(at) {
            panic!("assertion failed: self.is_char_boundary(at)");
        }
        String { vec: self.vec.split_off(at) }
    }
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Drain {
        let (start, end) = crate::ops::range_to_bounds(&range, self.len());
        if !self.as_str().is_char_boundary(start) || !self.as_str().is_char_boundary(end) {
            panic!("drain range is not on char boundaries");
        }
        let removed: Vec<u8> = self.vec.drain(start..end).collect();
        Drain { s: String { vec: removed }, pos: 0 }
    }
    pub fn replace_range<R: RangeBounds<usize>>(&mut self, range: R, replace_with: &str) {
        let (start, end) = crate::ops::range_to_bounds(&range, self.len());
        if !self.as_str().is_char_boundary(start) || !self.as_str().is_char_boundary(end) {
            panic!("replace_range is not on char boundaries");
        }
        let mut tail = self.vec.split_off(end);
        self.vec.truncate(start);
        self.vec.extend_from_slice(replace_with.as_bytes());
        self.vec.append(&mut tail);
    }
    pub fn into_boxed_str(self) -> crate::boxed::Box<str> {
        let b: crate::boxed::Box<[u8]> = self.vec.into_boxed_slice();
        unsafe { crate::intrinsics::transmute(b) }
    }
    pub fn leak<'a>(self) -> &'a mut str {
        unsafe { str::from_utf8_unchecked_mut(self.vec.leak()) }
    }
}

pub struct Drain {
    s: String,
    pos: usize,
}
impl Iterator for Drain {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if self.pos >= self.s.len() {
            return None;
        }
        let (c, n) = str::decode_at(self.s.as_bytes(), self.pos);
        self.pos += n;
        Some(c)
    }
}

impl Deref for String {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl DerefMut for String {
    fn deref_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl Clone for String {
    fn clone(&self) -> String {
        String { vec: self.vec.clone() }
    }
}
impl Default for String {
    fn default() -> String {
        String::new()
    }
}
impl fmt::Display for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}
impl fmt::Debug for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}
impl fmt::Write for String {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.push(c);
        Ok(())
    }
}
impl crate::hash::Hash for String {
    fn hash<H: crate::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl PartialEq for String {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}
impl Eq for String {}
impl PartialEq<str> for String {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}
impl PartialEq<&str> for String {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}
impl PartialEq<String> for str {
    fn eq(&self, other: &String) -> bool {
        self == other.as_str()
    }
}
impl PartialEq<String> for &str {
    fn eq(&self, other: &String) -> bool {
        *self == other.as_str()
    }
}
impl PartialOrd for String {
    fn partial_cmp(&self, other: &String) -> Option<Ordering> {
        Some(self.as_str().cmp(other.as_str()))
    }
}
impl Ord for String {
    fn cmp(&self, other: &String) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl From<&str> for String {
    fn from(s: &str) -> String {
        String { vec: s.as_bytes().to_vec() }
    }
}
impl From<&mut str> for String {
    fn from(s: &mut str) -> String {
        String { vec: s.as_bytes().to_vec() }
    }
}
impl From<&String> for String {
    fn from(s: &String) -> String {
        s.clone()
    }
}
impl From<char> for String {
    fn from(c: char) -> String {
        let mut s = String::new();
        s.push(c);
        s
    }
}
impl From<crate::boxed::Box<str>> for String {
    fn from(b: crate::boxed::Box<str>) -> String {
        b.into_string()
    }
}
impl From<String> for crate::boxed::Box<str> {
    fn from(s: String) -> crate::boxed::Box<str> {
        s.into_boxed_str()
    }
}
impl From<String> for Vec<u8> {
    fn from(s: String) -> Vec<u8> {
        s.vec
    }
}
impl<'a> From<Cow<'a, str>> for String {
    fn from(c: Cow<'a, str>) -> String {
        c.into_owned()
    }
}

impl AsRef<str> for String {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl AsRef<[u8]> for String {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl crate::borrow::Borrow<str> for String {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl crate::ops::Add<&str> for String {
    type Output = String;
    fn add(mut self, other: &str) -> String {
        self.push_str(other);
        self
    }
}
impl crate::ops::AddAssign<&str> for String {
    fn add_assign(&mut self, other: &str) {
        self.push_str(other);
    }
}

impl<I: crate::slice::SliceIndex<str>> crate::ops::Index<I> for String {
    type Output = I::Output;
    fn index(&self, index: I) -> &I::Output {
        index.index(self.as_str())
    }
}
impl<I: crate::slice::SliceIndex<str>> crate::ops::IndexMut<I> for String {
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        index.index_mut(self.as_mut_str())
    }
}

impl Extend<char> for String {
    fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) {
        for c in iter {
            self.push(c);
        }
    }
}
impl<'a> Extend<&'a char> for String {
    fn extend<I: IntoIterator<Item = &'a char>>(&mut self, iter: I) {
        for c in iter {
            self.push(*c);
        }
    }
}
impl<'a> Extend<&'a str> for String {
    fn extend<I: IntoIterator<Item = &'a str>>(&mut self, iter: I) {
        for s in iter {
            self.push_str(s);
        }
    }
}
impl Extend<String> for String {
    fn extend<I: IntoIterator<Item = String>>(&mut self, iter: I) {
        for s in iter {
            self.push_str(&s);
        }
    }
}
impl FromIterator<char> for String {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> String {
        let mut s = String::new();
        s.extend(iter);
        s
    }
}
impl<'a> FromIterator<&'a char> for String {
    fn from_iter<I: IntoIterator<Item = &'a char>>(iter: I) -> String {
        let mut s = String::new();
        s.extend(iter);
        s
    }
}
impl<'a> FromIterator<&'a str> for String {
    fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> String {
        let mut s = String::new();
        s.extend(iter);
        s
    }
}
impl FromIterator<String> for String {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> String {
        let mut s = String::new();
        s.extend(iter);
        s
    }
}

pub trait ToString {
    fn to_string(&self) -> String;
}

impl<T: fmt::Display + ?Sized> ToString for T {
    fn to_string(&self) -> String {
        let mut s = String::new();
        let mut f = fmt::Formatter::new(&mut s);
        let _ = fmt::Display::fmt(self, &mut f);
        s
    }
}
