//! `std::fs` over the rcc_os_* runtime (rt.c). `struct stat` / `dirent`
//! are decoded on the C side into plain integers (see `rcc_os_stat`).

use crate::ffi::OsString;
use crate::fmt;
use crate::io::{self, Error, ErrorKind, Read, Seek, SeekFrom, Write};
use crate::os::sys;
use crate::path::{Path, PathBuf};
use crate::string::String;
use crate::time::SystemTime;
use crate::vec::Vec;

pub(crate) fn pb(p: &Path) -> io::Result<(*const u8, usize)> {
    let b = p.bytes();
    if b.contains(&0) {
        return Err(Error::new_const(ErrorKind::InvalidInput, "file name contained an unexpected NUL byte"));
    }
    Ok((b.as_ptr(), b.len()))
}

fn cvt0(r: i32) -> io::Result<()> {
    if r < 0 { Err(Error::from_raw_os_error(-r)) } else { Ok(()) }
}

// ---------------------------------------------------------------------------
// File

pub struct File {
    fd: i32,
}

impl File {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
        OpenOptions::new().read(true).open(path.as_ref())
    }
    pub fn create<P: AsRef<Path>>(path: P) -> io::Result<File> {
        OpenOptions::new().write(true).create(true).truncate(true).open(path.as_ref())
    }
    pub fn create_new<P: AsRef<Path>>(path: P) -> io::Result<File> {
        OpenOptions::new().read(true).write(true).create_new(true).open(path.as_ref())
    }
    pub fn options() -> OpenOptions {
        OpenOptions::new()
    }
    pub(crate) fn from_raw(fd: i32) -> File {
        File { fd }
    }
    pub(crate) fn raw(&self) -> i32 {
        self.fd
    }
    pub(crate) fn into_raw(self) -> i32 {
        let fd = self.fd;
        crate::mem::forget(self);
        fd
    }
    pub fn sync_all(&self) -> io::Result<()> {
        cvt0(unsafe { sys::rcc_os_fsync(self.fd, 0) })
    }
    pub fn sync_data(&self) -> io::Result<()> {
        cvt0(unsafe { sys::rcc_os_fsync(self.fd, 1) })
    }
    pub fn set_len(&self, size: u64) -> io::Result<()> {
        cvt0(unsafe { sys::rcc_os_ftruncate(self.fd, size) })
    }
    pub fn metadata(&self) -> io::Result<Metadata> {
        let mut st = [0u64; 20];
        cvt0(unsafe { sys::rcc_os_fstat(self.fd, st.as_mut_ptr()) })?;
        Ok(Metadata { st })
    }
    pub fn try_clone(&self) -> io::Result<File> {
        let fd = sys::cvt32(unsafe { sys::rcc_os_dup(self.fd) })?;
        Ok(File { fd })
    }
    pub fn set_permissions(&self, perm: Permissions) -> io::Result<()> {
        cvt0(unsafe { sys::rcc_os_fchmod(self.fd, perm.mode) })
    }
    fn read_fd(&self, buf: &mut [u8]) -> io::Result<usize> {
        Ok(sys::cvt(unsafe { sys::rcc_os_read(self.fd, buf.as_mut_ptr(), buf.len()) })? as usize)
    }
    fn write_fd(&self, buf: &[u8]) -> io::Result<usize> {
        Ok(sys::cvt(unsafe { sys::rcc_os_write(self.fd, buf.as_ptr(), buf.len()) })? as usize)
    }
    fn seek_fd(&self, pos: SeekFrom) -> io::Result<u64> {
        let (off, whence) = match pos {
            SeekFrom::Start(n) => (n as i64, 0),
            SeekFrom::Current(n) => (n, 1),
            SeekFrom::End(n) => (n, 2),
        };
        Ok(sys::cvt(unsafe { sys::rcc_os_lseek(self.fd, off, whence) })? as u64)
    }
    fn read_to_end_fd(&self, buf: &mut Vec<u8>) -> io::Result<usize> {
        // size hint: the remaining length of the file, if known
        if let Ok(m) = self.metadata() {
            if let Ok(pos) = self.seek_fd(SeekFrom::Current(0)) {
                let len = m.len();
                if len > pos {
                    buf.reserve((len - pos) as usize + 1);
                }
            }
        }
        let mut me: &File = self;
        io::default_read_to_end(&mut me, buf)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        unsafe {
            sys::rcc_os_close(self.fd);
        }
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File").field("fd", &self.fd).finish()
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_fd(buf)
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.read_to_end_fd(buf)
    }
}
impl<'a> Read for &'a File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_fd(buf)
    }
}
impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_fd(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl<'a> Write for &'a File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_fd(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.seek_fd(pos)
    }
}
impl<'a> Seek for &'a File {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.seek_fd(pos)
    }
}
impl io::IsTerminal for File {
    fn is_terminal(&self) -> bool {
        unsafe { sys::rcc_os_isatty(self.fd) != 0 }
    }
}

// ---------------------------------------------------------------------------
// OpenOptions

#[derive(Clone)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
    pub(crate) custom_flags: i32,
    pub(crate) mode: u32,
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
            custom_flags: 0,
            mode: 0o666,
        }
    }
    pub fn read(&mut self, read: bool) -> &mut OpenOptions {
        self.read = read;
        self
    }
    pub fn write(&mut self, write: bool) -> &mut OpenOptions {
        self.write = write;
        self
    }
    pub fn append(&mut self, append: bool) -> &mut OpenOptions {
        self.append = append;
        self
    }
    pub fn truncate(&mut self, truncate: bool) -> &mut OpenOptions {
        self.truncate = truncate;
        self
    }
    pub fn create(&mut self, create: bool) -> &mut OpenOptions {
        self.create = create;
        self
    }
    pub fn create_new(&mut self, create_new: bool) -> &mut OpenOptions {
        self.create_new = create_new;
        self
    }
    pub fn open<P: AsRef<Path>>(&self, path: P) -> io::Result<File> {
        let mut flags = 0;
        if self.read {
            flags |= 1;
        }
        if self.write {
            flags |= 2;
        }
        if self.append {
            flags |= 4;
        }
        if self.truncate {
            flags |= 8;
        }
        if self.create {
            flags |= 16;
        }
        if self.create_new {
            flags |= 32;
        }
        let (p, n) = pb(path.as_ref())?;
        let fd = sys::cvt32(unsafe { sys::rcc_os_open(p, n, flags, self.custom_flags, self.mode) })?;
        Ok(File { fd })
    }
}

impl fmt::Debug for OpenOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OpenOptions(")?;
        f.debug_struct("OpenOptions")
            .field("read", &self.read)
            .field("write", &self.write)
            .field("append", &self.append)
            .field("truncate", &self.truncate)
            .field("create", &self.create)
            .field("create_new", &self.create_new)
            .field("custom_flags", &self.custom_flags)
            .field("mode", &format_args!("0o{:06o}", self.mode))
            .finish()?;
        f.write_str(")")
    }
}

// ---------------------------------------------------------------------------
// Metadata, FileType, Permissions

/// `st` is the array filled by `rcc_os_stat` (layout documented in rt.c).
#[derive(Clone)]
pub struct Metadata {
    pub(crate) st: [u64; 20],
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FileType {
    pub(crate) kind: u8,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Permissions {
    pub(crate) mode: u32,
}

fn time_from(secs: u64, nanos: u64) -> SystemTime {
    SystemTime::from_parts(secs as i64, nanos as u32)
}

impl Metadata {
    pub fn file_type(&self) -> FileType {
        FileType { kind: self.st[0] as u8 }
    }
    pub fn is_dir(&self) -> bool {
        self.st[0] == 2
    }
    pub fn is_file(&self) -> bool {
        self.st[0] == 1
    }
    pub fn is_symlink(&self) -> bool {
        self.st[0] == 3
    }
    pub fn len(&self) -> u64 {
        self.st[2]
    }
    pub fn permissions(&self) -> Permissions {
        Permissions { mode: self.st[1] as u32 }
    }
    pub fn modified(&self) -> io::Result<SystemTime> {
        Ok(time_from(self.st[3], self.st[4]))
    }
    pub fn accessed(&self) -> io::Result<SystemTime> {
        Ok(time_from(self.st[5], self.st[6]))
    }
    pub fn created(&self) -> io::Result<SystemTime> {
        if self.st[11] != 0 {
            Ok(time_from(self.st[9], self.st[10]))
        } else {
            Err(Error::new_const(ErrorKind::Unsupported, "creation time is not available on this platform currently"))
        }
    }
}

impl fmt::Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("Metadata");
        d.field("file_type", &self.file_type());
        d.field("permissions", &self.permissions());
        d.field("len", &self.len());
        d.field("modified", &self.modified());
        d.field("accessed", &self.accessed());
        d.field("created", &self.created());
        d.finish_non_exhaustive()
    }
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        self.kind == 2
    }
    pub fn is_file(&self) -> bool {
        self.kind == 1
    }
    pub fn is_symlink(&self) -> bool {
        self.kind == 3
    }
    pub(crate) fn is_block_device(&self) -> bool {
        self.kind == 4
    }
    pub(crate) fn is_char_device(&self) -> bool {
        self.kind == 5
    }
    pub(crate) fn is_fifo(&self) -> bool {
        self.kind == 6
    }
    pub(crate) fn is_socket(&self) -> bool {
        self.kind == 7
    }
}

impl fmt::Debug for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileType")
            .field("is_file", &self.is_file())
            .field("is_dir", &self.is_dir())
            .field("is_symlink", &self.is_symlink())
            .finish_non_exhaustive()
    }
}

impl Permissions {
    pub fn readonly(&self) -> bool {
        self.mode & 0o222 == 0
    }
    pub fn set_readonly(&mut self, readonly: bool) {
        if readonly {
            self.mode &= !0o222;
        } else {
            self.mode |= 0o222;
        }
    }
}

impl fmt::Debug for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode = self.mode;
        write!(f, "Permissions(FilePermissions {{ mode: 0o{:06o}", mode)?;
        let t = match mode & 0o170000 {
            0o040000 => 'd',
            0o020000 => 'c',
            0o060000 => 'b',
            0o010000 => 'p',
            0o120000 => 'l',
            0o100000 => '-',
            _ => ' ',
        };
        if t != ' ' {
            let bit = |m: u32, c: char| if mode & m != 0 { c } else { '-' };
            let x = |exec: u32, special: u32, on: char, off: char| match (mode & exec != 0, mode & special != 0) {
                (true, true) => on,
                (false, true) => off,
                (true, false) => 'x',
                (false, false) => '-',
            };
            write!(
                f,
                " ({}{}{}{}{}{}{}{}{}{})",
                t,
                bit(0o400, 'r'),
                bit(0o200, 'w'),
                x(0o100, 0o4000, 's', 'S'),
                bit(0o040, 'r'),
                bit(0o020, 'w'),
                x(0o010, 0o2000, 's', 'S'),
                bit(0o004, 'r'),
                bit(0o002, 'w'),
                x(0o001, 0o1000, 't', 'T')
            )?;
        }
        f.write_str(" })")
    }
}

// ---------------------------------------------------------------------------
// directories

pub struct ReadDir {
    dir: usize,
    root: PathBuf,
    end: bool,
}

pub struct DirEntry {
    root: PathBuf,
    name: Vec<u8>,
    kind: i32,
    ino: u64,
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;
    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        if self.end {
            return None;
        }
        let mut name: *const u8 = crate::ptr::null();
        let mut len = 0usize;
        let mut kind = 0i32;
        let mut ino = 0u64;
        let r = unsafe { sys::rcc_os_readdir(self.dir, &mut name, &mut len, &mut kind, &mut ino) };
        if r < 0 {
            self.end = true;
            return Some(Err(Error::from_raw_os_error(-r)));
        }
        if r == 0 {
            self.end = true;
            return None;
        }
        let bytes = unsafe { crate::slice::from_raw_parts(name, len) }.to_vec();
        Some(Ok(DirEntry { root: self.root.clone(), name: bytes, kind, ino }))
    }
}

impl Drop for ReadDir {
    fn drop(&mut self) {
        unsafe { sys::rcc_os_closedir(self.dir) }
    }
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ReadDir").field(&self.root).finish()
    }
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.root.join(Path::from_bytes(&self.name))
    }
    pub fn file_name(&self) -> OsString {
        OsString::from_vec(self.name.clone())
    }
    pub fn metadata(&self) -> io::Result<Metadata> {
        symlink_metadata(self.path())
    }
    pub fn file_type(&self) -> io::Result<FileType> {
        if self.kind != 0 {
            Ok(FileType { kind: self.kind as u8 })
        } else {
            Ok(self.metadata()?.file_type())
        }
    }
    pub(crate) fn ino(&self) -> u64 {
        self.ino
    }
}

impl fmt::Debug for DirEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DirEntry").field(&self.path()).finish()
    }
}

pub fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<ReadDir> {
    let path = path.as_ref();
    let (p, n) = pb(path)?;
    let mut err = 0i32;
    let dir = unsafe { sys::rcc_os_opendir(p, n, &mut err) };
    if dir == 0 {
        return Err(Error::from_raw_os_error(err));
    }
    Ok(ReadDir { dir, root: path.to_path_buf(), end: false })
}

#[derive(Debug)]
pub struct DirBuilder {
    recursive: bool,
    pub(crate) mode: u32,
}

impl DirBuilder {
    pub fn new() -> DirBuilder {
        DirBuilder { recursive: false, mode: 0o777 }
    }
    pub fn recursive(&mut self, recursive: bool) -> &mut DirBuilder {
        self.recursive = recursive;
        self
    }
    pub fn create<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        if self.recursive { self.create_dir_all(path.as_ref()) } else { self.mkdir(path.as_ref()) }
    }
    fn mkdir(&self, path: &Path) -> io::Result<()> {
        let (p, n) = pb(path)?;
        cvt0(unsafe { sys::rcc_os_mkdir(p, n, self.mode) })
    }
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        if path.bytes().is_empty() {
            return Ok(());
        }
        match self.mkdir(path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                if e.kind() != ErrorKind::NotFound {
                    if path.is_dir() {
                        return Ok(());
                    }
                    return Err(e);
                }
            }
        }
        match path.parent() {
            Some(p) => self.create_dir_all(p)?,
            None => return Err(Error::new_const(ErrorKind::Uncategorized, "failed to create whole tree")),
        }
        match self.mkdir(path) {
            Ok(()) => Ok(()),
            Err(e) => {
                if path.is_dir() { Ok(()) } else { Err(e) }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// free functions

pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut f = File::open(path)?;
    let mut v = Vec::new();
    f.read_to_end(&mut v)?;
    Ok(v)
}

pub fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut f = File::open(path)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}

pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let mut f = File::create(path)?;
    f.write_all(contents.as_ref())
}

pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    let (p, n) = pb(path.as_ref())?;
    let mut st = [0u64; 20];
    cvt0(unsafe { sys::rcc_os_stat(p, n, 1, st.as_mut_ptr()) })?;
    Ok(Metadata { st })
}

pub fn symlink_metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    let (p, n) = pb(path.as_ref())?;
    let mut st = [0u64; 20];
    cvt0(unsafe { sys::rcc_os_stat(p, n, 0, st.as_mut_ptr()) })?;
    Ok(Metadata { st })
}

pub fn exists<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    match metadata(path) {
        Ok(_) => Ok(true),
        Err(e) => {
            if e.kind() == ErrorKind::NotFound { Ok(false) } else { Err(e) }
        }
    }
}

pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let (p, n) = pb(path.as_ref())?;
    cvt0(unsafe { sys::rcc_os_unlink(p, n) })
}

pub fn remove_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let (p, n) = pb(path.as_ref())?;
    cvt0(unsafe { sys::rcc_os_rmdir(p, n) })
}

fn remove_dir_all_recursive(path: &Path) -> io::Result<()> {
    for child in read_dir(path)? {
        let child = child?;
        let child_path = child.path();
        let r = if child.file_type()?.is_dir() { remove_dir_all_recursive(&child_path) } else { remove_file(&child_path) };
        if let Err(e) = r {
            if e.kind() != ErrorKind::NotFound {
                return Err(e);
            }
        }
    }
    remove_dir(path)
}

pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();
    let m = symlink_metadata(path)?;
    if m.file_type().is_symlink() { remove_file(path) } else { remove_dir_all_recursive(path) }
}

pub fn create_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    DirBuilder::new().create(path)
}

pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    DirBuilder::new().recursive(true).create(path)
}

pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    let (a, an) = pb(from.as_ref())?;
    let (b, bn) = pb(to.as_ref())?;
    cvt0(unsafe { sys::rcc_os_path2(0, a, an, b, bn) })
}

pub fn hard_link<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    let (a, an) = pb(original.as_ref())?;
    let (b, bn) = pb(link.as_ref())?;
    cvt0(unsafe { sys::rcc_os_path2(1, a, an, b, bn) })
}

pub(crate) fn symlink_impl(original: &Path, link: &Path) -> io::Result<()> {
    let (a, an) = pb(original)?;
    let (b, bn) = pb(link)?;
    cvt0(unsafe { sys::rcc_os_path2(2, a, an, b, bn) })
}

pub fn soft_link<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    symlink_impl(original.as_ref(), link.as_ref())
}

pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<u64> {
    let mut reader = File::open(from)?;
    let m = reader.metadata()?;
    if !m.is_file() {
        return Err(Error::new_const(
            ErrorKind::InvalidInput,
            "the source path is neither a regular file nor a symlink to a regular file",
        ));
    }
    let perm = m.permissions();
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    opts.mode = perm.mode & 0o7777;
    let mut writer = opts.open(to)?;
    writer.set_permissions(perm)?;
    io::copy(&mut reader, &mut writer)
}

pub fn canonicalize<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let (p, n) = pb(path.as_ref())?;
    let mut out: *mut u8 = crate::ptr::null_mut();
    let r = unsafe { sys::rcc_os_path_query(0, p, n, &mut out) };
    Ok(PathBuf::from_vec(sys::c_buf_result(r, out)?))
}

pub fn read_link<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let (p, n) = pb(path.as_ref())?;
    let mut out: *mut u8 = crate::ptr::null_mut();
    let r = unsafe { sys::rcc_os_path_query(1, p, n, &mut out) };
    Ok(PathBuf::from_vec(sys::c_buf_result(r, out)?))
}

pub fn set_permissions<P: AsRef<Path>>(path: P, perm: Permissions) -> io::Result<()> {
    let (p, n) = pb(path.as_ref())?;
    cvt0(unsafe { sys::rcc_os_chmod(p, n, perm.mode) })
}
