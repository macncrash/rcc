//! Minimal `CString` / `CStr` (NUL-terminated byte strings). `CStr` uses
//! the same view scheme as `OsStr`.

use crate::fmt;
use crate::ops::Deref;
use crate::string::String;
use crate::vec::Vec;

/// A NUL-terminated owned byte string without interior NULs.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CString {
    inner: Vec<u8>, // includes the trailing NUL
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NulError(usize, Vec<u8>);

impl NulError {
    pub fn nul_position(&self) -> usize {
        self.0
    }
    pub fn into_vec(self) -> Vec<u8> {
        self.1
    }
}
impl fmt::Display for NulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "nul byte found in provided data at position: {}", self.0)
    }
}
impl crate::error::Error for NulError {}

impl From<NulError> for crate::io::Error {
    fn from(_: NulError) -> crate::io::Error {
        crate::io::Error::new_const(crate::io::ErrorKind::InvalidInput, "data provided contains a nul byte")
    }
}

impl CString {
    pub fn new<T: Into<Vec<u8>>>(t: T) -> Result<CString, NulError> {
        let mut v: Vec<u8> = t.into();
        let mut i = 0;
        while i < v.len() {
            if v[i] == 0 {
                return Err(NulError(i, v));
            }
            i += 1;
        }
        v.push(0);
        Ok(CString { inner: v })
    }
    pub fn as_ptr(&self) -> *const i8 {
        self.inner.as_ptr() as *const i8
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner[..self.inner.len() - 1]
    }
    pub fn as_bytes_with_nul(&self) -> &[u8] {
        &self.inner
    }
    pub fn into_bytes(mut self) -> Vec<u8> {
        self.inner.pop();
        self.inner
    }
    pub fn into_bytes_with_nul(self) -> Vec<u8> {
        self.inner
    }
    pub fn into_string(self) -> Result<String, crate::string::FromUtf8Error> {
        String::from_utf8(self.into_bytes())
    }
    pub fn as_c_str(&self) -> &CStr {
        CStr::from_view(&self.inner)
    }
}
impl Deref for CString {
    type Target = CStr;
    fn deref(&self) -> &CStr {
        self.as_c_str()
    }
}
impl fmt::Debug for CString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_c_str(), f)
    }
}

/// Borrowed C string: a (leaked) view of bytes including the NUL.
pub struct CStr {
    inner: &'static [u8],
}

impl CStr {
    fn from_view<'a>(b: &'a [u8]) -> &'a CStr {
        crate::boxed::Box::leak(crate::boxed::Box::new(CStr { inner: super::os_str::erase(b) }))
    }
    pub unsafe fn from_ptr<'a>(p: *const i8) -> &'a CStr {
        let n = crate::os::sys::c_strlen(p as *const u8);
        CStr::from_view(crate::slice::from_raw_parts(p as *const u8, n + 1))
    }
    pub fn from_bytes_with_nul(bytes: &[u8]) -> Result<&CStr, FromBytesWithNulError> {
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0 {
                if i + 1 == bytes.len() {
                    return Ok(CStr::from_view(bytes));
                }
                return Err(FromBytesWithNulError(()));
            }
            i += 1;
        }
        Err(FromBytesWithNulError(()))
    }
    pub fn as_ptr(&self) -> *const i8 {
        self.inner.as_ptr() as *const i8
    }
    pub fn to_bytes(&self) -> &[u8] {
        &self.inner[..self.inner.len() - 1]
    }
    pub fn to_bytes_with_nul(&self) -> &[u8] {
        self.inner
    }
    pub fn to_str(&self) -> Result<&str, crate::str::Utf8Error> {
        crate::str::from_utf8(self.to_bytes())
    }
    pub fn to_string_lossy(&self) -> crate::borrow::Cow<'_, str> {
        String::from_utf8_lossy(self.to_bytes())
    }
    pub fn count_bytes(&self) -> usize {
        self.inner.len() - 1
    }
    pub fn is_empty(&self) -> bool {
        self.inner.len() == 1
    }
    pub fn to_owned(&self) -> CString {
        CString { inner: self.inner.to_vec() }
    }
}
impl fmt::Debug for CStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        for &b in self.to_bytes() {
            if b == b'"' || b == b'\\' {
                write!(f, "\\{}", b as char)?;
            } else if b >= 0x20 && b < 0x7f {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{:02x}", b)?;
            }
        }
        f.write_str("\"")
    }
}
impl PartialEq for CStr {
    fn eq(&self, other: &CStr) -> bool {
        self.inner == other.inner
    }
}
impl Eq for CStr {}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FromBytesWithNulError(());
impl fmt::Display for FromBytesWithNulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("data provided is not nul terminated or contains an interior nul byte")
    }
}
