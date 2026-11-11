//! `std::path` with unix semantics (`/` separator, no prefixes).
//!
//! Design (rcc has no unsized struct tails, so `Path` cannot be a
//! dynamically sized newtype over `OsStr`):
//!
//! - `Path` is a sized *view* `{ inner: &[u8] }` and `&Path` is a thin
//!   pointer to such a view. Every API that hands out a borrowed `&Path` /
//!   `&OsStr` for some bytes (`Path::new`, `parent`, `file_name`,
//!   `strip_prefix`, components, `AsRef<Path> for str`, ...) allocates a
//!   16-byte view and leaks it; the bytes themselves are never copied.
//! - `PathBuf` owns a `Vec<u8>` and embeds a `Cell<Path>` view of it that
//!   `Deref` refreshes, so `&PathBuf -> &Path` (autoderef) allocates nothing.
//! - Internal algorithms (comparison, hashing, `starts_with`, `push`, ...)
//!   work on raw byte slices and do not allocate views.
//! The component parser is a port of real std's `Components` (front/back
//! state machine), so `parent`, `file_name`, `components().as_path()` etc.
//! return exactly the same sub-slices as real std.

use crate::borrow::{Borrow, Cow, ToOwned};
use crate::cell::Cell;
use crate::cmp::Ordering;
use crate::ffi::os_str::{debug_bytes, display_bytes, erase};
use crate::ffi::{OsStr, OsString};
use crate::fmt;
use crate::fs;
use crate::hash::{Hash, Hasher};
use crate::io;
use crate::iter::FusedIterator;
use crate::ops::Deref;
use crate::str::FromStr;
use crate::string::String;
use crate::vec::Vec;

pub const MAIN_SEPARATOR: char = '/';
pub const MAIN_SEPARATOR_STR: &str = "/";

pub fn is_separator(c: char) -> bool {
    c == '/'
}

// ---------------------------------------------------------------------------
// Prefix (never produced on unix; kept so that matches on Component compile)

#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Prefix<'a> {
    Verbatim(&'a OsStr),
    VerbatimUNC(&'a OsStr, &'a OsStr),
    VerbatimDisk(u8),
    DeviceNS(&'a OsStr),
    UNC(&'a OsStr, &'a OsStr),
    Disk(u8),
}

impl<'a> Prefix<'a> {
    pub fn is_verbatim(&self) -> bool {
        match self {
            Prefix::Verbatim(_) | Prefix::VerbatimDisk(_) | Prefix::VerbatimUNC(..) => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct PrefixComponent<'a> {
    raw: &'a OsStr,
    parsed: Prefix<'a>,
}

impl<'a> PrefixComponent<'a> {
    pub fn kind(&self) -> Prefix<'a> {
        self.parsed
    }
    pub fn as_os_str(&self) -> &'a OsStr {
        self.raw
    }
}

// ---------------------------------------------------------------------------
// Components

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Component<'a> {
    Prefix(PrefixComponent<'a>),
    RootDir,
    CurDir,
    ParentDir,
    Normal(&'a OsStr),
}

impl<'a> Component<'a> {
    pub fn as_os_str(self) -> &'a OsStr {
        match self {
            Component::Prefix(p) => p.as_os_str(),
            Component::RootDir => OsStr::from_bytes_view(b"/"),
            Component::CurDir => OsStr::from_bytes_view(b"."),
            Component::ParentDir => OsStr::from_bytes_view(b".."),
            Component::Normal(path) => path,
        }
    }
}

impl<'a> AsRef<OsStr> for Component<'a> {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}
impl<'a> AsRef<Path> for Component<'a> {
    fn as_ref(&self) -> &Path {
        Path::from_bytes(self.as_os_str().bytes())
    }
}

/// A component as a byte slice (no view allocation).
#[derive(Copy, Clone, PartialEq, Eq)]
enum Raw<'a> {
    Root,
    Cur,
    Parent,
    Normal(&'a [u8]),
}

impl<'a> Raw<'a> {
    fn rank(&self) -> u8 {
        match self {
            Raw::Root => 1,
            Raw::Cur => 2,
            Raw::Parent => 3,
            Raw::Normal(_) => 4,
        }
    }
    fn cmp(&self, other: &Raw<'a>) -> Ordering {
        match (self, other) {
            (Raw::Normal(a), Raw::Normal(b)) => a.cmp(b),
            _ => self.rank().cmp(&other.rank()),
        }
    }
    fn bytes(&self) -> &'a [u8] {
        match self {
            Raw::Root => b"/",
            Raw::Cur => b".",
            Raw::Parent => b"..",
            Raw::Normal(b) => b,
        }
    }
    fn to_component(self) -> Component<'a> {
        match self {
            Raw::Root => Component::RootDir,
            Raw::Cur => Component::CurDir,
            Raw::Parent => Component::ParentDir,
            Raw::Normal(b) => Component::Normal(OsStr::from_bytes_view(b)),
        }
    }
}

const ST_PREFIX: u8 = 0;
const ST_START_DIR: u8 = 1;
const ST_BODY: u8 = 2;
const ST_DONE: u8 = 3;

#[derive(Clone)]
pub struct Components<'a> {
    path: &'a [u8],
    has_physical_root: bool,
    front: u8,
    back: u8,
}

fn components_of<'a>(b: &'a [u8]) -> Components<'a> {
    Components { path: b, has_physical_root: !b.is_empty() && b[0] == b'/', front: ST_PREFIX, back: ST_BODY }
}

impl<'a> Components<'a> {
    fn len_before_body(&self) -> usize {
        let root = if self.front <= ST_START_DIR && self.has_physical_root { 1 } else { 0 };
        let cur_dir = if self.front <= ST_START_DIR && self.include_cur_dir() { 1 } else { 0 };
        root + cur_dir
    }
    fn finished(&self) -> bool {
        self.front == ST_DONE || self.back == ST_DONE || self.front > self.back
    }
    fn include_cur_dir(&self) -> bool {
        if self.has_physical_root {
            return false;
        }
        let p = self.path;
        if p.is_empty() || p[0] != b'.' {
            return false;
        }
        p.len() == 1 || p[1] == b'/'
    }
    fn parse_single(&self, comp: &'a [u8]) -> Option<Raw<'a>> {
        if comp.is_empty() || comp == b"." {
            None
        } else if comp == b".." {
            Some(Raw::Parent)
        } else {
            Some(Raw::Normal(comp))
        }
    }
    fn parse_next(&self) -> (usize, Option<Raw<'a>>) {
        let p = self.path;
        let mut i = 0;
        while i < p.len() && p[i] != b'/' {
            i += 1;
        }
        let (extra, comp) = if i == p.len() { (0, p) } else { (1, &p[..i]) };
        (comp.len() + extra, self.parse_single(comp))
    }
    fn parse_next_back(&self) -> (usize, Option<Raw<'a>>) {
        let start = self.len_before_body();
        let p = &self.path[start..];
        let mut i = p.len();
        while i > 0 && p[i - 1] != b'/' {
            i -= 1;
        }
        let (extra, comp) = if i == 0 { (0, p) } else { (1, &p[i..]) };
        (comp.len() + extra, self.parse_single(comp))
    }
    fn trim_left(&mut self) {
        while !self.path.is_empty() {
            let (size, comp) = self.parse_next();
            if comp.is_some() {
                return;
            }
            self.path = &self.path[size..];
        }
    }
    fn trim_right(&mut self) {
        while self.path.len() > self.len_before_body() {
            let (size, comp) = self.parse_next_back();
            if comp.is_some() {
                return;
            }
            self.path = &self.path[..self.path.len() - size];
        }
    }
    fn rest_bytes(&self) -> &'a [u8] {
        let mut comps = self.clone();
        if comps.front == ST_BODY {
            comps.trim_left();
        }
        if comps.back == ST_BODY {
            comps.trim_right();
        }
        comps.path
    }
    pub fn as_path(&self) -> &'a Path {
        Path::from_bytes(self.rest_bytes())
    }

    fn next_raw(&mut self) -> Option<Raw<'a>> {
        while !self.finished() {
            if self.front == ST_PREFIX {
                self.front = ST_START_DIR;
            } else if self.front == ST_START_DIR {
                self.front = ST_BODY;
                if self.has_physical_root {
                    self.path = &self.path[1..];
                    return Some(Raw::Root);
                } else if self.include_cur_dir() {
                    self.path = &self.path[1..];
                    return Some(Raw::Cur);
                }
            } else if self.front == ST_BODY {
                if self.path.is_empty() {
                    self.front = ST_DONE;
                } else {
                    let (size, comp) = self.parse_next();
                    self.path = &self.path[size..];
                    if comp.is_some() {
                        return comp;
                    }
                }
            } else {
                return None;
            }
        }
        None
    }

    fn next_back_raw(&mut self) -> Option<Raw<'a>> {
        while !self.finished() {
            if self.back == ST_BODY {
                if self.path.len() > self.len_before_body() {
                    let (size, comp) = self.parse_next_back();
                    self.path = &self.path[..self.path.len() - size];
                    if comp.is_some() {
                        return comp;
                    }
                } else {
                    self.back = ST_START_DIR;
                }
            } else if self.back == ST_START_DIR {
                self.back = ST_PREFIX;
                if self.has_physical_root {
                    self.path = &self.path[..self.path.len() - 1];
                    return Some(Raw::Root);
                } else if self.include_cur_dir() {
                    self.path = &self.path[..self.path.len() - 1];
                    return Some(Raw::Cur);
                }
            } else if self.back == ST_PREFIX {
                self.back = ST_DONE;
                return None;
            } else {
                return None;
            }
        }
        None
    }
}

impl<'a> Iterator for Components<'a> {
    type Item = Component<'a>;
    fn next(&mut self) -> Option<Component<'a>> {
        match self.next_raw() {
            Some(r) => Some(r.to_component()),
            None => None,
        }
    }
}
impl<'a> DoubleEndedIterator for Components<'a> {
    fn next_back(&mut self) -> Option<Component<'a>> {
        match self.next_back_raw() {
            Some(r) => Some(r.to_component()),
            None => None,
        }
    }
}
impl<'a> FusedIterator for Components<'a> {}

impl<'a> PartialEq for Components<'a> {
    fn eq(&self, other: &Components<'a>) -> bool {
        raw_cmp(self.clone(), other.clone()) == Ordering::Equal
    }
}
impl<'a> Eq for Components<'a> {}

impl<'a> fmt::Debug for Components<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Components(")?;
        f.debug_list().entries(self.clone()).finish()?;
        f.write_str(")")
    }
}

impl<'a> AsRef<Path> for Components<'a> {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}
impl<'a> AsRef<OsStr> for Components<'a> {
    fn as_ref(&self) -> &OsStr {
        self.as_path().as_os_str()
    }
}

fn raw_cmp<'a, 'b>(mut a: Components<'a>, mut b: Components<'b>) -> Ordering {
    loop {
        match (a.next_raw(), b.next_raw()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let c = match (x, y) {
                    (Raw::Normal(p), Raw::Normal(q)) => p.cmp(q),
                    _ => x.rank().cmp(&y.rank()),
                };
                if c != Ordering::Equal {
                    return c;
                }
            }
        }
    }
}

/// Iterate `iter` past `prefix` (both raw); `None` if `prefix` is not a
/// prefix of `iter`.
fn iter_after<'a, 'b>(mut iter: Components<'a>, mut prefix: Components<'b>) -> Option<Components<'a>> {
    loop {
        let mut next = iter.clone();
        match (next.next_raw(), prefix.next_raw()) {
            (Some(x), Some(y)) => {
                if x.bytes() != y.bytes() || x.rank() != y.rank() {
                    return None;
                }
            }
            (Some(_), None) => return Some(iter),
            (None, None) => return Some(iter),
            (None, Some(_)) => return None,
        }
        iter = next;
    }
}

fn iter_after_back<'a, 'b>(mut iter: Components<'a>, mut suffix: Components<'b>) -> bool {
    loop {
        match (iter.next_back_raw(), suffix.next_back_raw()) {
            (Some(x), Some(y)) => {
                if x.bytes() != y.bytes() || x.rank() != y.rank() {
                    return false;
                }
            }
            (_, None) => return true,
            (None, Some(_)) => return false,
        }
    }
}

pub struct Iter<'a> {
    inner: Components<'a>,
}
impl<'a> Clone for Iter<'a> {
    fn clone(&self) -> Iter<'a> {
        Iter { inner: self.inner.clone() }
    }
}
impl<'a> Iter<'a> {
    pub fn as_path(&self) -> &'a Path {
        self.inner.as_path()
    }
}
impl<'a> Iterator for Iter<'a> {
    type Item = &'a OsStr;
    fn next(&mut self) -> Option<&'a OsStr> {
        match self.inner.next_raw() {
            Some(r) => Some(OsStr::from_bytes_view(r.bytes())),
            None => None,
        }
    }
}
impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<&'a OsStr> {
        match self.inner.next_back_raw() {
            Some(r) => Some(OsStr::from_bytes_view(r.bytes())),
            None => None,
        }
    }
}
impl<'a> FusedIterator for Iter<'a> {}
impl<'a> fmt::Debug for Iter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Iter(")?;
        f.debug_list().entries(self.clone()).finish()?;
        f.write_str(")")
    }
}

pub struct Ancestors<'a> {
    next: Option<&'a Path>,
}
impl<'a> Iterator for Ancestors<'a> {
    type Item = &'a Path;
    fn next(&mut self) -> Option<&'a Path> {
        let next = self.next;
        self.next = match next {
            Some(p) => p.parent(),
            None => None,
        };
        next
    }
}
impl<'a> FusedIterator for Ancestors<'a> {}

// ---------------------------------------------------------------------------
// Path

pub struct Path {
    inner: &'static [u8],
}

/// `(before, after)` of the last `.` in a file name, as real std's
/// `rsplit_file_at_dot`.
fn rsplit_file_at_dot(file: &[u8]) -> (Option<&[u8]>, Option<&[u8]>) {
    if file == b".." {
        return (Some(file), None);
    }
    let mut i = file.len();
    while i > 0 && file[i - 1] != b'.' {
        i -= 1;
    }
    if i == 0 {
        return (None, Some(file));
    }
    let before = &file[..i - 1];
    let after = &file[i..];
    if before.is_empty() {
        (Some(file), None)
    } else {
        (Some(before), Some(after))
    }
}

impl Path {
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &Path {
        Path::from_bytes(s.as_ref().bytes())
    }
    /// A (leaked) view of `b`.
    pub(crate) fn from_bytes<'a>(b: &'a [u8]) -> &'a Path {
        crate::boxed::Box::leak(crate::boxed::Box::new(Path { inner: erase(b) }))
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        self.inner
    }
    fn comps(&self) -> Components<'_> {
        components_of(self.inner)
    }
    fn file_name_raw(&self) -> Option<&[u8]> {
        match self.comps().next_back_raw() {
            Some(Raw::Normal(p)) => Some(p),
            _ => None,
        }
    }
    fn parent_raw(&self) -> Option<&[u8]> {
        let mut comps = self.comps();
        match comps.next_back_raw() {
            Some(Raw::Normal(_)) | Some(Raw::Cur) | Some(Raw::Parent) => Some(comps.rest_bytes()),
            _ => None,
        }
    }
    fn file_stem_raw(&self) -> Option<&[u8]> {
        match self.file_name_raw() {
            Some(name) => {
                let (before, after) = rsplit_file_at_dot(name);
                match before {
                    Some(b) => Some(b),
                    None => after,
                }
            }
            None => None,
        }
    }

    pub fn as_os_str(&self) -> &OsStr {
        OsStr::from_bytes_view(self.inner)
    }
    pub fn to_str(&self) -> Option<&str> {
        crate::str::from_utf8(self.inner).ok()
    }
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.inner)
    }
    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf::from_vec(self.inner.to_vec())
    }
    pub fn is_absolute(&self) -> bool {
        self.has_root()
    }
    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }
    pub fn has_root(&self) -> bool {
        !self.inner.is_empty() && self.inner[0] == b'/'
    }
    pub fn parent(&self) -> Option<&Path> {
        match self.parent_raw() {
            Some(b) => Some(Path::from_bytes(b)),
            None => None,
        }
    }
    pub fn ancestors(&self) -> Ancestors<'_> {
        Ancestors { next: Some(self) }
    }
    pub fn file_name(&self) -> Option<&OsStr> {
        match self.file_name_raw() {
            Some(b) => Some(OsStr::from_bytes_view(b)),
            None => None,
        }
    }
    pub fn strip_prefix<P: AsRef<Path>>(&self, base: P) -> Result<&Path, StripPrefixError> {
        match iter_after(self.comps(), base.as_ref().comps()) {
            Some(c) => Ok(Path::from_bytes(c.rest_bytes())),
            None => Err(StripPrefixError(())),
        }
    }
    pub fn starts_with<P: AsRef<Path>>(&self, base: P) -> bool {
        iter_after(self.comps(), base.as_ref().comps()).is_some()
    }
    pub fn ends_with<P: AsRef<Path>>(&self, child: P) -> bool {
        iter_after_back(self.comps(), child.as_ref().comps())
    }
    pub fn file_stem(&self) -> Option<&OsStr> {
        match self.file_stem_raw() {
            Some(b) => Some(OsStr::from_bytes_view(b)),
            None => None,
        }
    }
    pub fn file_prefix(&self) -> Option<&OsStr> {
        match self.file_name_raw() {
            Some(name) => {
                if name == b".." {
                    return Some(OsStr::from_bytes_view(name));
                }
                let start = if !name.is_empty() && name[0] == b'.' { 1 } else { 0 };
                let mut i = start;
                while i < name.len() && name[i] != b'.' {
                    i += 1;
                }
                Some(OsStr::from_bytes_view(&name[..i]))
            }
            None => None,
        }
    }
    pub fn extension(&self) -> Option<&OsStr> {
        match self.file_name_raw() {
            Some(name) => {
                let (before, after) = rsplit_file_at_dot(name);
                match (before, after) {
                    (Some(_), Some(a)) => Some(OsStr::from_bytes_view(a)),
                    _ => None,
                }
            }
            None => None,
        }
    }
    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let mut buf = self.to_path_buf();
        buf.push_bytes(path.as_ref().inner);
        buf
    }
    pub fn with_file_name<S: AsRef<OsStr>>(&self, file_name: S) -> PathBuf {
        let mut buf = self.to_path_buf();
        buf.set_file_name(file_name);
        buf
    }
    pub fn with_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf {
        let mut buf = self.to_path_buf();
        buf.set_extension(extension);
        buf
    }
    pub fn components(&self) -> Components<'_> {
        self.comps()
    }
    pub fn iter(&self) -> Iter<'_> {
        Iter { inner: self.comps() }
    }
    pub fn display(&self) -> Display<'_> {
        Display { path: self }
    }
    pub fn metadata(&self) -> io::Result<fs::Metadata> {
        fs::metadata(self)
    }
    pub fn symlink_metadata(&self) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(self)
    }
    pub fn canonicalize(&self) -> io::Result<PathBuf> {
        fs::canonicalize(self)
    }
    pub fn read_link(&self) -> io::Result<PathBuf> {
        fs::read_link(self)
    }
    pub fn read_dir(&self) -> io::Result<fs::ReadDir> {
        fs::read_dir(self)
    }
    pub fn exists(&self) -> bool {
        fs::metadata(self).is_ok()
    }
    pub fn try_exists(&self) -> io::Result<bool> {
        fs::exists(self)
    }
    pub fn is_file(&self) -> bool {
        match fs::metadata(self) {
            Ok(m) => m.is_file(),
            Err(_) => false,
        }
    }
    pub fn is_dir(&self) -> bool {
        match fs::metadata(self) {
            Ok(m) => m.is_dir(),
            Err(_) => false,
        }
    }
    pub fn is_symlink(&self) -> bool {
        match fs::symlink_metadata(self) {
            Ok(m) => m.file_type().is_symlink(),
            Err(_) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StripPrefixError(());

impl fmt::Display for StripPrefixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("prefix not found")
    }
}
impl crate::error::Error for StripPrefixError {}

pub struct Display<'a> {
    path: &'a Path,
}
impl<'a> fmt::Display for Display<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display_bytes(self.path.inner, f)
    }
}
impl<'a> fmt::Debug for Display<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_bytes(self.path.inner, f)
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_bytes(self.inner, f)
    }
}

fn path_eq(a: &[u8], b: &[u8]) -> bool {
    a == b || raw_cmp(components_of(a), components_of(b)) == Ordering::Equal
}
fn path_cmp(a: &[u8], b: &[u8]) -> Ordering {
    if a == b { Ordering::Equal } else { raw_cmp(components_of(a), components_of(b)) }
}
fn path_hash<H: Hasher>(b: &[u8], state: &mut H) {
    let mut c = components_of(b);
    let mut n = 0usize;
    loop {
        match c.next_raw() {
            Some(r) => {
                state.write_u8(r.rank());
                state.write(r.bytes());
                n += 1;
            }
            None => break,
        }
    }
    state.write_usize(n);
}

impl PartialEq for Path {
    fn eq(&self, other: &Path) -> bool {
        path_eq(self.inner, other.inner)
    }
}
impl Eq for Path {}
impl PartialOrd for Path {
    fn partial_cmp(&self, other: &Path) -> Option<Ordering> {
        Some(path_cmp(self.inner, other.inner))
    }
}
impl Ord for Path {
    fn cmp(&self, other: &Path) -> Ordering {
        path_cmp(self.inner, other.inner)
    }
}
impl Hash for Path {
    fn hash<H: Hasher>(&self, state: &mut H) {
        path_hash(self.inner, state)
    }
}

impl AsRef<Path> for Path {
    fn as_ref(&self) -> &Path {
        self
    }
}
impl AsRef<Path> for str {
    fn as_ref(&self) -> &Path {
        Path::from_bytes(self.as_bytes())
    }
}
impl AsRef<Path> for String {
    fn as_ref(&self) -> &Path {
        Path::from_bytes(self.as_bytes())
    }
}
impl AsRef<Path> for OsStr {
    fn as_ref(&self) -> &Path {
        Path::from_bytes(self.bytes())
    }
}
impl AsRef<Path> for OsString {
    fn as_ref(&self) -> &Path {
        Path::from_bytes(self.as_vec())
    }
}
impl<'a> AsRef<Path> for Cow<'a, OsStr> {
    fn as_ref(&self) -> &Path {
        Path::from_bytes(self.bytes())
    }
}
impl AsRef<Path> for PathBuf {
    fn as_ref(&self) -> &Path {
        self.view()
    }
}
impl AsRef<OsStr> for Path {
    fn as_ref(&self) -> &OsStr {
        OsStr::from_bytes_view(self.inner)
    }
}
impl AsRef<OsStr> for PathBuf {
    fn as_ref(&self) -> &OsStr {
        OsStr::from_bytes_view(&self.inner)
    }
}

impl ToOwned for Path {
    type Owned = PathBuf;
    fn to_owned(&self) -> PathBuf {
        self.to_path_buf()
    }
}

impl<'a> IntoIterator for &'a Path {
    type Item = &'a OsStr;
    type IntoIter = Iter<'a>;
    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

// ---------------------------------------------------------------------------
// PathBuf

pub struct PathBuf {
    inner: Vec<u8>,
    view: Cell<Path>,
}

fn empty_path_view() -> Path {
    Path { inner: erase(b"") }
}

impl PathBuf {
    pub fn new() -> PathBuf {
        PathBuf { inner: Vec::new(), view: Cell::new(empty_path_view()) }
    }
    pub fn with_capacity(capacity: usize) -> PathBuf {
        PathBuf::from_vec(Vec::with_capacity(capacity))
    }
    pub(crate) fn from_vec(v: Vec<u8>) -> PathBuf {
        PathBuf { inner: v, view: Cell::new(empty_path_view()) }
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.inner
    }
    fn view(&self) -> &Path {
        self.view.set(Path { inner: erase(self.inner.as_slice()) });
        unsafe { &*self.view.as_ptr() }
    }
    pub fn as_path(&self) -> &Path {
        self.view()
    }
    fn push_bytes(&mut self, path: &[u8]) {
        let need_sep = match self.inner.last() {
            Some(c) => *c != b'/',
            None => false,
        };
        if !path.is_empty() && path[0] == b'/' {
            self.inner.truncate(0);
        } else if need_sep {
            self.inner.push(b'/');
        }
        self.inner.extend_from_slice(path);
    }
    pub fn push<P: AsRef<Path>>(&mut self, path: P) {
        let p = path.as_ref();
        self.push_bytes(p.inner);
    }
    pub fn pop(&mut self) -> bool {
        let len = match self.view().parent_raw() {
            Some(p) => Some(p.len()),
            None => None,
        };
        match len {
            Some(n) => {
                self.inner.truncate(n);
                true
            }
            None => false,
        }
    }
    pub fn set_file_name<S: AsRef<OsStr>>(&mut self, file_name: S) {
        if self.view().file_name_raw().is_some() {
            self.pop();
        }
        let name = file_name.as_ref();
        self.push_bytes(name.bytes());
    }
    pub fn set_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        let end = match self.view().file_stem_raw() {
            None => return false,
            Some(stem) => {
                let base = self.inner.as_ptr() as usize;
                (stem.as_ptr() as usize) - base + stem.len()
            }
        };
        self.inner.truncate(end);
        let new = extension.as_ref().bytes();
        if !new.is_empty() {
            self.inner.push(b'.');
            self.inner.extend_from_slice(new);
        }
        true
    }
    pub fn add_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        if self.view().file_name_raw().is_none() {
            return false;
        }
        let new = extension.as_ref().bytes();
        if !new.is_empty() {
            // drop trailing separators / `.` so the extension attaches to the name
            let keep = {
                let v = self.view();
                let mut c = v.comps();
                c.trim_right();
                c.path.len()
            };
            self.inner.truncate(keep);
            self.inner.push(b'.');
            self.inner.extend_from_slice(new);
        }
        true
    }
    pub fn into_os_string(self) -> OsString {
        OsString::from_vec(self.inner)
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
    pub fn shrink_to_fit(&mut self) {}
}

impl Deref for PathBuf {
    type Target = Path;
    fn deref(&self) -> &Path {
        self.view()
    }
}
impl Borrow<Path> for PathBuf {
    fn borrow(&self) -> &Path {
        self.view()
    }
}
impl Clone for PathBuf {
    fn clone(&self) -> PathBuf {
        PathBuf::from_vec(self.inner.clone())
    }
}
impl Default for PathBuf {
    fn default() -> PathBuf {
        PathBuf::new()
    }
}
impl fmt::Debug for PathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_bytes(&self.inner, f)
    }
}
impl PartialEq for PathBuf {
    fn eq(&self, other: &PathBuf) -> bool {
        path_eq(&self.inner, &other.inner)
    }
}
impl Eq for PathBuf {}
impl PartialOrd for PathBuf {
    fn partial_cmp(&self, other: &PathBuf) -> Option<Ordering> {
        Some(path_cmp(&self.inner, &other.inner))
    }
}
impl Ord for PathBuf {
    fn cmp(&self, other: &PathBuf) -> Ordering {
        path_cmp(&self.inner, &other.inner)
    }
}
impl Hash for PathBuf {
    fn hash<H: Hasher>(&self, state: &mut H) {
        path_hash(&self.inner, state)
    }
}

impl PartialEq<Path> for PathBuf {
    fn eq(&self, other: &Path) -> bool {
        path_eq(&self.inner, other.inner)
    }
}
impl PartialEq<PathBuf> for Path {
    fn eq(&self, other: &PathBuf) -> bool {
        path_eq(self.inner, &other.inner)
    }
}
impl<'a> PartialEq<&'a Path> for PathBuf {
    fn eq(&self, other: &&'a Path) -> bool {
        path_eq(&self.inner, other.inner)
    }
}
impl<'a> PartialEq<PathBuf> for &'a Path {
    fn eq(&self, other: &PathBuf) -> bool {
        path_eq(self.inner, &other.inner)
    }
}
impl PartialEq<OsStr> for Path {
    fn eq(&self, other: &OsStr) -> bool {
        path_eq(self.inner, other.bytes())
    }
}
impl<'a> PartialEq<Cow<'a, Path>> for Path {
    fn eq(&self, other: &Cow<'a, Path>) -> bool {
        path_eq(self.inner, other.bytes())
    }
}
impl PartialOrd<PathBuf> for Path {
    fn partial_cmp(&self, other: &PathBuf) -> Option<Ordering> {
        Some(path_cmp(self.inner, &other.inner))
    }
}
impl PartialOrd<Path> for PathBuf {
    fn partial_cmp(&self, other: &Path) -> Option<Ordering> {
        Some(path_cmp(&self.inner, other.inner))
    }
}

impl From<String> for PathBuf {
    fn from(s: String) -> PathBuf {
        PathBuf::from_vec(s.into_bytes())
    }
}
impl From<OsString> for PathBuf {
    fn from(s: OsString) -> PathBuf {
        PathBuf::from_vec(s.into_vec())
    }
}
impl<'a, T: ?Sized + AsRef<OsStr>> From<&'a T> for PathBuf {
    fn from(s: &'a T) -> PathBuf {
        PathBuf::from_vec(s.as_ref().bytes().to_vec())
    }
}
impl<'a> From<Cow<'a, Path>> for PathBuf {
    fn from(s: Cow<'a, Path>) -> PathBuf {
        s.into_owned()
    }
}
impl From<PathBuf> for OsString {
    fn from(p: PathBuf) -> OsString {
        OsString::from_vec(p.inner)
    }
}
impl<'a> From<&'a Path> for Cow<'a, Path> {
    fn from(p: &'a Path) -> Cow<'a, Path> {
        Cow::Borrowed(p)
    }
}
impl<'a> From<PathBuf> for Cow<'a, Path> {
    fn from(p: PathBuf) -> Cow<'a, Path> {
        Cow::Owned(p)
    }
}
impl FromStr for PathBuf {
    type Err = crate::convert::Infallible;
    fn from_str(s: &str) -> Result<PathBuf, crate::convert::Infallible> {
        Ok(PathBuf::from(s))
    }
}
impl<P: AsRef<Path>> crate::iter::FromIterator<P> for PathBuf {
    fn from_iter<I: IntoIterator<Item = P>>(iter: I) -> PathBuf {
        let mut buf = PathBuf::new();
        for p in iter {
            buf.push(p);
        }
        buf
    }
}
impl<P: AsRef<Path>> crate::iter::Extend<P> for PathBuf {
    fn extend<I: IntoIterator<Item = P>>(&mut self, iter: I) {
        for p in iter {
            self.push(p);
        }
    }
}
impl<'a> IntoIterator for &'a PathBuf {
    type Item = &'a OsStr;
    type IntoIter = Iter<'a>;
    fn into_iter(self) -> Iter<'a> {
        self.view().iter()
    }
}

pub fn absolute<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let path = path.as_ref();
    if path.inner.is_empty() {
        return Err(io::Error::new_const(io::ErrorKind::InvalidInput, "cannot make an empty path absolute"));
    }
    // like real std on unix: normalise `.` and repeated separators, keep `..`
    let mut out = if path.is_absolute() { PathBuf::new() } else { crate::env::current_dir()? };
    let mut c = path.comps();
    let mut leading_slashes = 0;
    while leading_slashes < path.inner.len() && path.inner[leading_slashes] == b'/' {
        leading_slashes += 1;
    }
    if leading_slashes == 2 {
        out.push_bytes(b"//");
    }
    loop {
        match c.next_raw() {
            Some(Raw::Root) => {
                if leading_slashes != 2 {
                    out.push_bytes(b"/")
                }
            }
            Some(Raw::Cur) => {}
            Some(r) => out.push_bytes(r.bytes()),
            None => break,
        }
    }
    if path.inner.len() > 1 && path.inner[path.inner.len() - 1] == b'/' {
        out.inner.push(b'/');
    }
    Ok(out)
}
