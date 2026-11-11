//! `OsStr` / `OsString` (see the design note in ffi/mod.rs).

use crate::borrow::{Borrow, Cow, ToOwned};
use crate::cell::Cell;
use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::ops::Deref;
use crate::str;
use crate::string::String;
use crate::vec::Vec;

/// Borrowed OS string: a sized view of bytes; `&OsStr` is a thin pointer
/// to a view.
pub struct OsStr {
    inner: &'static [u8],
}

/// Erase the lifetime of a byte slice (the views are lifetime-erased; the
/// public API hands them out with the lifetime of the borrowed data).
pub(crate) fn erase<'a>(b: &'a [u8]) -> &'static [u8] {
    unsafe { crate::slice::from_raw_parts(b.as_ptr(), b.len()) }
}

impl OsStr {
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &OsStr {
        s.as_ref()
    }
    /// A (leaked) view of `b`.
    pub(crate) fn from_bytes_view<'a>(b: &'a [u8]) -> &'a OsStr {
        crate::boxed::Box::leak(crate::boxed::Box::new(OsStr { inner: erase(b) }))
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        self.inner
    }
    pub unsafe fn from_encoded_bytes_unchecked(bytes: &[u8]) -> &OsStr {
        OsStr::from_bytes_view(bytes)
    }
    pub fn as_encoded_bytes(&self) -> &[u8] {
        self.inner
    }
    pub fn to_str(&self) -> Option<&str> {
        str::from_utf8(self.inner).ok()
    }
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.inner)
    }
    pub fn to_os_string(&self) -> OsString {
        OsString::from_vec(self.inner.to_vec())
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_ascii(&self) -> bool {
        self.inner.is_ascii()
    }
    pub fn to_ascii_lowercase(&self) -> OsString {
        let mut v = self.inner.to_vec();
        for b in v.iter_mut() {
            *b = b.to_ascii_lowercase();
        }
        OsString::from_vec(v)
    }
    pub fn to_ascii_uppercase(&self) -> OsString {
        let mut v = self.inner.to_vec();
        for b in v.iter_mut() {
            *b = b.to_ascii_uppercase();
        }
        OsString::from_vec(v)
    }
    pub fn eq_ignore_ascii_case<S: AsRef<OsStr>>(&self, other: S) -> bool {
        self.inner.eq_ignore_ascii_case(other.as_ref().inner)
    }
    pub fn display(&self) -> Display<'_> {
        Display { inner: self }
    }
}

/// Writes `bytes` like `str`'s Debug (with quotes), invalid UTF-8 bytes as
/// `\xNN`.
pub(crate) fn debug_bytes(bytes: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if let Ok(s) = str::from_utf8(bytes) {
        return fmt::Debug::fmt(s, f);
    }
    f.write_str("\"")?;
    let mut rest = bytes;
    while !rest.is_empty() {
        let (valid, bad) = match str::from_utf8(rest) {
            Ok(s) => (s, 0),
            Err(e) => {
                let n = e.valid_up_to();
                let bad = invalid_len(&rest[n..]);
                (unsafe { str::from_utf8_unchecked(&rest[..n]) }, bad)
            }
        };
        let quoted = crate::fmt::format(format_args!("{:?}", valid));
        f.write_str(&quoted[1..quoted.len() - 1])?;
        let start = valid.len();
        for b in &rest[start..start + bad] {
            f.write_fmt(format_args!("\\x{:02X}", b))?;
        }
        rest = &rest[start + bad..];
    }
    f.write_str("\"")
}

/// Writes `bytes` like `str`'s Display (honouring width), lossily.
pub(crate) fn display_bytes(bytes: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match str::from_utf8(bytes) {
        Ok(s) => f.pad(s),
        Err(_) => {
            let s = String::from_utf8_lossy(bytes);
            f.pad(&s)
        }
    }
}

pub struct Display<'a> {
    inner: &'a OsStr,
}
impl<'a> fmt::Display for Display<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display_bytes(self.inner.inner, f)
    }
}
impl<'a> fmt::Debug for Display<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_bytes(self.inner.inner, f)
    }
}

impl fmt::Debug for OsStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_bytes(self.inner, f)
    }
}

impl PartialEq for OsStr {
    fn eq(&self, other: &OsStr) -> bool {
        self.inner == other.inner
    }
}
impl Eq for OsStr {}
impl PartialEq<str> for OsStr {
    fn eq(&self, other: &str) -> bool {
        self.inner == other.as_bytes()
    }
}
impl PartialEq<OsStr> for str {
    fn eq(&self, other: &OsStr) -> bool {
        self.as_bytes() == other.inner
    }
}
impl<'a> PartialEq<&'a str> for OsStr {
    fn eq(&self, other: &&'a str) -> bool {
        self.inner == other.as_bytes()
    }
}
impl PartialEq<OsString> for OsStr {
    fn eq(&self, other: &OsString) -> bool {
        self.inner == other.inner.as_slice()
    }
}
impl<'a> PartialEq<OsString> for &'a OsStr {
    fn eq(&self, other: &OsString) -> bool {
        self.inner == other.inner.as_slice()
    }
}
impl PartialOrd for OsStr {
    fn partial_cmp(&self, other: &OsStr) -> Option<Ordering> {
        Some(self.inner.cmp(other.inner))
    }
}
impl Ord for OsStr {
    fn cmp(&self, other: &OsStr) -> Ordering {
        self.inner.cmp(other.inner)
    }
}
impl Hash for OsStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.inner);
        state.write_u8(0xff);
    }
}

impl AsRef<OsStr> for OsStr {
    fn as_ref(&self) -> &OsStr {
        self
    }
}
impl AsRef<OsStr> for str {
    fn as_ref(&self) -> &OsStr {
        OsStr::from_bytes_view(self.as_bytes())
    }
}
impl AsRef<OsStr> for String {
    fn as_ref(&self) -> &OsStr {
        OsStr::from_bytes_view(self.as_bytes())
    }
}
impl AsRef<OsStr> for OsString {
    fn as_ref(&self) -> &OsStr {
        self.view()
    }
}

impl ToOwned for OsStr {
    type Owned = OsString;
    fn to_owned(&self) -> OsString {
        self.to_os_string()
    }
}

impl<'a> Default for &'a OsStr {
    fn default() -> &'a OsStr {
        OsStr::from_bytes_view(b"")
    }
}

// ---------------------------------------------------------------------------

/// Owned OS string. `view` caches the `OsStr` view handed out by `Deref`.
pub struct OsString {
    inner: Vec<u8>,
    view: Cell<OsStr>,
}

fn empty_view() -> OsStr {
    OsStr { inner: erase(b"") }
}

impl OsString {
    pub fn new() -> OsString {
        OsString { inner: Vec::new(), view: Cell::new(empty_view()) }
    }
    pub(crate) fn from_vec(v: Vec<u8>) -> OsString {
        OsString { inner: v, view: Cell::new(empty_view()) }
    }
    pub(crate) fn into_vec(self) -> Vec<u8> {
        self.inner
    }
    pub(crate) fn as_vec(&self) -> &Vec<u8> {
        &self.inner
    }
    pub(crate) fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.inner
    }
    fn view(&self) -> &OsStr {
        self.view.set(OsStr { inner: erase(self.inner.as_slice()) });
        unsafe { &*self.view.as_ptr() }
    }
    pub unsafe fn from_encoded_bytes_unchecked(bytes: Vec<u8>) -> OsString {
        OsString::from_vec(bytes)
    }
    pub fn into_encoded_bytes(self) -> Vec<u8> {
        self.inner
    }
    pub fn with_capacity(capacity: usize) -> OsString {
        OsString::from_vec(Vec::with_capacity(capacity))
    }
    pub fn as_os_str(&self) -> &OsStr {
        self.view()
    }
    pub fn into_string(self) -> Result<String, OsString> {
        match String::from_utf8(self.inner) {
            Ok(s) => Ok(s),
            Err(e) => Err(OsString::from_vec(e.into_bytes())),
        }
    }
    pub fn push<T: AsRef<OsStr>>(&mut self, s: T) {
        self.inner.extend_from_slice(s.as_ref().inner);
    }
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }
    pub fn clear(&mut self) {
        self.inner.clear()
    }
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional)
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Deref for OsString {
    type Target = OsStr;
    fn deref(&self) -> &OsStr {
        self.view()
    }
}
impl Borrow<OsStr> for OsString {
    fn borrow(&self) -> &OsStr {
        self.view()
    }
}
impl Clone for OsString {
    fn clone(&self) -> OsString {
        OsString::from_vec(self.inner.clone())
    }
}
impl Default for OsString {
    fn default() -> OsString {
        OsString::new()
    }
}
impl fmt::Debug for OsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_bytes(&self.inner, f)
    }
}
impl PartialEq for OsString {
    fn eq(&self, other: &OsString) -> bool {
        self.inner == other.inner
    }
}
impl Eq for OsString {}
impl PartialEq<str> for OsString {
    fn eq(&self, other: &str) -> bool {
        self.inner.as_slice() == other.as_bytes()
    }
}
impl<'a> PartialEq<&'a str> for OsString {
    fn eq(&self, other: &&'a str) -> bool {
        self.inner.as_slice() == other.as_bytes()
    }
}
impl PartialEq<OsString> for str {
    fn eq(&self, other: &OsString) -> bool {
        self.as_bytes() == other.inner.as_slice()
    }
}
impl<'a> PartialEq<OsString> for &'a str {
    fn eq(&self, other: &OsString) -> bool {
        self.as_bytes() == other.inner.as_slice()
    }
}
impl PartialEq<OsStr> for OsString {
    fn eq(&self, other: &OsStr) -> bool {
        self.inner.as_slice() == other.inner
    }
}
impl<'a> PartialEq<&'a OsStr> for OsString {
    fn eq(&self, other: &&'a OsStr) -> bool {
        self.inner.as_slice() == other.inner
    }
}
impl PartialOrd for OsString {
    fn partial_cmp(&self, other: &OsString) -> Option<Ordering> {
        Some(self.inner.cmp(&other.inner))
    }
}
impl Ord for OsString {
    fn cmp(&self, other: &OsString) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}
impl Hash for OsString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.inner);
        state.write_u8(0xff);
    }
}

impl From<String> for OsString {
    fn from(s: String) -> OsString {
        OsString::from_vec(s.into_bytes())
    }
}
impl<'a> From<&'a str> for OsString {
    fn from(s: &'a str) -> OsString {
        OsString::from_vec(s.as_bytes().to_vec())
    }
}
impl<'a> From<&'a String> for OsString {
    fn from(s: &'a String) -> OsString {
        OsString::from_vec(s.as_bytes().to_vec())
    }
}
impl<'a> From<&'a OsStr> for OsString {
    fn from(s: &'a OsStr) -> OsString {
        s.to_os_string()
    }
}
impl<'a> From<&'a OsString> for OsString {
    fn from(s: &'a OsString) -> OsString {
        s.clone()
    }
}
impl<'a> From<Cow<'a, OsStr>> for OsString {
    fn from(s: Cow<'a, OsStr>) -> OsString {
        s.into_owned()
    }
}
impl crate::str::FromStr for OsString {
    type Err = crate::convert::Infallible;
    fn from_str(s: &str) -> Result<OsString, crate::convert::Infallible> {
        Ok(OsString::from(s))
    }
}
impl<'a> crate::iter::Extend<&'a OsStr> for OsString {
    fn extend<I: IntoIterator<Item = &'a OsStr>>(&mut self, iter: I) {
        for s in iter {
            self.push(s);
        }
    }
}

/// Length of the maximal invalid UTF-8 prefix of `b` (b is not valid at 0).
fn invalid_len(b: &[u8]) -> usize {
    let b0 = b[0];
    let (need, lo, hi) = match b0 {
        0xC2..=0xDF => (1, 0x80, 0xBF),
        0xE0 => (2, 0xA0, 0xBF),
        0xE1..=0xEC | 0xEE..=0xEF => (2, 0x80, 0xBF),
        0xED => (2, 0x80, 0x9F),
        0xF0 => (3, 0x90, 0xBF),
        0xF1..=0xF3 => (3, 0x80, 0xBF),
        0xF4 => (3, 0x80, 0x8F),
        _ => return 1,
    };
    let mut k = 1;
    while k <= need && k < b.len() {
        let (l, h) = if k == 1 { (lo, hi) } else { (0x80, 0xBF) };
        if b[k] < l || b[k] > h {
            break;
        }
        k += 1;
    }
    k
}
