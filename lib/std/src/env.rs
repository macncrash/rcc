//! `std::env`: arguments (captured by `rcc_rt_init` at program start),
//! environment variables, working directory, executable path, temp dir.

use crate::ffi::{OsStr, OsString};
use crate::fmt;
use crate::io;
use crate::os::sys;
use crate::path::{Path, PathBuf};
use crate::string::String;
use crate::vec::{self, Vec};

extern "C" {
    fn rcc_args_count() -> usize;
    fn rcc_args_get(i: usize, len: *mut usize) -> *const u8;
}

fn raw_args() -> Vec<OsString> {
    let n = unsafe { rcc_args_count() };
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        let mut len = 0usize;
        let p = unsafe { rcc_args_get(i, &mut len) };
        let bytes = unsafe { crate::slice::from_raw_parts(p, len) };
        v.push(OsString::from_vec(bytes.to_vec()));
    }
    v
}

pub struct Args {
    inner: vec::IntoIter<String>,
}
pub struct ArgsOs {
    inner: vec::IntoIter<OsString>,
}

pub fn args() -> Args {
    let mut v = Vec::new();
    for a in raw_args() {
        match a.into_string() {
            Ok(s) => v.push(s),
            Err(a) => panic!("called `Result::unwrap()` on an `Err` value: {:?}", a),
        }
    }
    Args { inner: v.into_iter() }
}

pub fn args_os() -> ArgsOs {
    ArgsOs { inner: raw_args().into_iter() }
}

impl Iterator for Args {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl DoubleEndedIterator for Args {
    fn next_back(&mut self) -> Option<String> {
        self.inner.next_back()
    }
}
impl ExactSizeIterator for Args {}
impl fmt::Debug for Args {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Args").field("args", &self.inner.as_slice()).finish()
    }
}

impl Iterator for ArgsOs {
    type Item = OsString;
    fn next(&mut self) -> Option<OsString> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl DoubleEndedIterator for ArgsOs {
    fn next_back(&mut self) -> Option<OsString> {
        self.inner.next_back()
    }
}
impl ExactSizeIterator for ArgsOs {}
impl fmt::Debug for ArgsOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArgsOs").field("args", &self.inner.as_slice()).finish()
    }
}

// ---------------------------------------------------------------------------
// variables

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VarError {
    NotPresent,
    NotUnicode(OsString),
}

impl fmt::Display for VarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VarError::NotPresent => f.write_str("environment variable not found"),
            VarError::NotUnicode(s) => write!(f, "environment variable was not valid unicode: {:?}", s),
        }
    }
}
impl crate::error::Error for VarError {}

pub fn var_os<K: AsRef<OsStr>>(key: K) -> Option<OsString> {
    let k = key.as_ref().bytes();
    let mut out: *mut u8 = crate::ptr::null_mut();
    let n = unsafe { sys::rcc_os_getenv(k.as_ptr(), k.len(), &mut out) };
    if n < 0 {
        None
    } else {
        Some(OsString::from_vec(sys::take_c_buf(out, n as usize)))
    }
}

pub fn var<K: AsRef<OsStr>>(key: K) -> Result<String, VarError> {
    match var_os(key) {
        Some(s) => match s.into_string() {
            Ok(s) => Ok(s),
            Err(s) => Err(VarError::NotUnicode(s)),
        },
        None => Err(VarError::NotPresent),
    }
}

pub fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    let k = key.as_ref().bytes();
    let v = value.as_ref().bytes();
    let r = unsafe { sys::rcc_os_setenv(k.as_ptr(), k.len(), v.as_ptr(), v.len()) };
    if r != 0 {
        panic!(
            "failed to set environment variable `{:?}` to `{:?}`: {}",
            key.as_ref(),
            value.as_ref(),
            io::Error::from_raw_os_error(-r)
        );
    }
}

pub fn remove_var<K: AsRef<OsStr>>(key: K) {
    let k = key.as_ref().bytes();
    let r = unsafe { sys::rcc_os_unsetenv(k.as_ptr(), k.len()) };
    if r != 0 {
        panic!("failed to remove environment variable `{:?}`: {}", key.as_ref(), io::Error::from_raw_os_error(-r));
    }
}

fn env_snapshot() -> Vec<(OsString, OsString)> {
    let mut out: *mut u8 = crate::ptr::null_mut();
    let n = unsafe { sys::rcc_os_environ(&mut out) };
    let block = sys::take_c_buf(out, n);
    let mut res = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < block.len() {
        if block[i] == 0 {
            let entry = &block[start..i];
            // like real std: skip a leading '=' when looking for the separator
            if !entry.is_empty() {
                let mut j = 1;
                while j < entry.len() && entry[j] != b'=' {
                    j += 1;
                }
                if j < entry.len() {
                    res.push((OsString::from_vec(entry[..j].to_vec()), OsString::from_vec(entry[j + 1..].to_vec())));
                }
            }
            start = i + 1;
        }
        i += 1;
    }
    res
}

pub struct Vars {
    inner: vec::IntoIter<(OsString, OsString)>,
}
pub struct VarsOs {
    inner: vec::IntoIter<(OsString, OsString)>,
}

pub fn vars() -> Vars {
    Vars { inner: env_snapshot().into_iter() }
}
pub fn vars_os() -> VarsOs {
    VarsOs { inner: env_snapshot().into_iter() }
}

impl Iterator for Vars {
    type Item = (String, String);
    fn next(&mut self) -> Option<(String, String)> {
        match self.inner.next() {
            Some((k, v)) => Some((k.into_string().unwrap(), v.into_string().unwrap())),
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl Iterator for VarsOs {
    type Item = (OsString, OsString);
    fn next(&mut self) -> Option<(OsString, OsString)> {
        self.inner.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl fmt::Debug for Vars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Vars { .. }")
    }
}
impl fmt::Debug for VarsOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("VarsOs { .. }")
    }
}

// ---------------------------------------------------------------------------
// directories and paths

pub fn current_dir() -> io::Result<PathBuf> {
    let mut out: *mut u8 = crate::ptr::null_mut();
    let n = unsafe { sys::rcc_os_getcwd(&mut out) };
    Ok(PathBuf::from_vec(sys::c_buf_result(n, out)?))
}

pub fn set_current_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let b = path.as_ref().bytes();
    let r = unsafe { sys::rcc_os_chdir(b.as_ptr(), b.len()) };
    if r < 0 { Err(io::Error::from_raw_os_error(-r)) } else { Ok(()) }
}

pub fn current_exe() -> io::Result<PathBuf> {
    let mut out: *mut u8 = crate::ptr::null_mut();
    let n = unsafe { sys::rcc_os_current_exe(&mut out) };
    Ok(PathBuf::from_vec(sys::c_buf_result(n, out)?))
}

pub fn temp_dir() -> PathBuf {
    match var_os("TMPDIR") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from("/tmp"),
    }
}

pub fn home_dir() -> Option<PathBuf> {
    match var_os("HOME") {
        Some(p) => Some(PathBuf::from(p)),
        None => None,
    }
}

pub struct SplitPaths<'a> {
    rest: Option<&'a [u8]>,
}

pub fn split_paths<T: AsRef<OsStr> + ?Sized>(unparsed: &T) -> SplitPaths<'_> {
    SplitPaths { rest: Some(unparsed.as_ref().bytes()) }
}

impl<'a> Iterator for SplitPaths<'a> {
    type Item = PathBuf;
    fn next(&mut self) -> Option<PathBuf> {
        match self.rest {
            None => None,
            Some(b) => {
                let mut i = 0;
                while i < b.len() && b[i] != b':' {
                    i += 1;
                }
                if i < b.len() {
                    self.rest = Some(&b[i + 1..]);
                } else {
                    self.rest = None;
                }
                Some(PathBuf::from_vec(b[..i].to_vec()))
            }
        }
    }
}

#[derive(Debug)]
pub struct JoinPathsError;

impl fmt::Display for JoinPathsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("path segment contains separator `:`")
    }
}
impl crate::error::Error for JoinPathsError {}

pub fn join_paths<I, T>(paths: I) -> Result<OsString, JoinPathsError>
where
    I: IntoIterator<Item = T>,
    T: AsRef<OsStr>,
{
    let mut out = Vec::new();
    let mut first = true;
    for p in paths {
        let b = p.as_ref().bytes();
        if b.contains(&b':') {
            return Err(JoinPathsError);
        }
        if !first {
            out.push(b':');
        }
        first = false;
        out.extend_from_slice(b);
    }
    Ok(OsString::from_vec(out))
}

pub mod consts {
    pub const ARCH: &str = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else {
        "unknown"
    };
    pub const OS: &str = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    };
    pub const FAMILY: &str = "unix";
    pub const EXE_SUFFIX: &str = "";
    pub const EXE_EXTENSION: &str = "";
    pub const DLL_PREFIX: &str = "lib";
    pub const DLL_SUFFIX: &str = if cfg!(target_os = "macos") { ".dylib" } else { ".so" };
    pub const DLL_EXTENSION: &str = if cfg!(target_os = "macos") { "dylib" } else { "so" };
}
