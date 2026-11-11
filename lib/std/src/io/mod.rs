//! `std::io`: the Read/Write/BufRead/Seek traits, `Error`, stdio,
//! buffered readers/writers, `Cursor` and the small utilities.
//!
//! `_print`/`_eprint` are the targets of `print!`/`eprint!` (expander
//! contract). Stdout goes through C stdio with LineWriter semantics (see
//! `rcc_os_stdout_write` in rt.c): complete lines are flushed at once, a
//! trailing partial line stays buffered until the next newline, an explicit
//! flush, `process::exit`, a panic or the end of `main`. Stderr is
//! unbuffered.

use crate::cmp;
use crate::fmt;
use crate::os::sys;
use crate::string::String;
use crate::vec::Vec;

mod buffered;
mod cursor;
mod error;
mod impls;
mod stdio;
mod util;

pub use self::buffered::{BufReader, BufWriter, IntoInnerError, LineWriter};
pub use self::cursor::Cursor;
pub use self::error::{Error, ErrorKind, Result};
pub use self::stdio::{stderr, stdin, stdout, IsTerminal, Stderr, StderrLock, Stdin, StdinLock, Stdout, StdoutLock};
pub use self::util::{copy, empty, repeat, sink, Empty, Repeat, Sink};

pub mod prelude {
    pub use super::{BufRead, Read, Seek, Write};
}

extern "C" {
    fn rcc_write(fd: i32, buf: *const u8, len: usize) -> i32;
}

/// Writer to file descriptor 2 that flushes stdout first (used by panic
/// messages in rt.rs; the runtime owns it).
pub(crate) struct RawFd {
    pub(crate) fd: i32,
}

impl fmt::Write for RawFd {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let r = unsafe { rcc_write(self.fd, s.as_ptr(), s.len()) };
        if r == 0 { Ok(()) } else { Err(fmt::Error) }
    }
}

pub fn _print(args: fmt::Arguments<'_>) {
    if let Err(e) = stdout().write_fmt(args) {
        crate::rt::panic_fmt(format_args!("failed printing to stdout: {}", e), file!(), line!(), column!());
    }
}

pub fn _eprint(args: fmt::Arguments<'_>) {
    if let Err(e) = stderr().write_fmt(args) {
        crate::rt::panic_fmt(format_args!("failed printing to stderr: {}", e), file!(), line!(), column!());
    }
}

// ---------------------------------------------------------------------------
// Read

const DEFAULT_BUF_SIZE: usize = 8 * 1024;

fn is_interrupted(e: &Error) -> bool {
    e.kind() == ErrorKind::Interrupted
}

pub(crate) fn default_read_to_end<R: Read + ?Sized>(r: &mut R, buf: &mut Vec<u8>) -> Result<usize> {
    let start = buf.len();
    loop {
        if buf.capacity() - buf.len() < 32 {
            let grow = cmp::max(buf.capacity(), 64);
            buf.reserve(cmp::min(grow, 1 << 22));
        }
        let len = buf.len();
        let spare = buf.capacity() - len;
        let dst = unsafe { crate::slice::from_raw_parts_mut(buf.as_mut_ptr().add(len), spare) };
        match r.read(dst) {
            Ok(0) => return Ok(buf.len() - start),
            Ok(n) => unsafe { buf.set_len(len + n) },
            Err(e) => {
                if !is_interrupted(&e) {
                    return Err(e);
                }
            }
        }
    }
}

fn invalid_utf8() -> Error {
    Error::new_const(ErrorKind::InvalidData, "stream did not contain valid UTF-8")
}

/// Run `f` on the bytes of `buf`; keep what it appended only if that is
/// valid UTF-8 (as real std's `append_to_string`).
pub(crate) fn append_to_string<F: FnOnce(&mut Vec<u8>) -> Result<usize>>(buf: &mut String, f: F) -> Result<usize> {
    let v = unsafe { buf.as_mut_vec() };
    let old = v.len();
    let ret = f(v);
    if crate::str::from_utf8(&v[old..]).is_err() {
        v.truncate(old);
        match ret {
            Ok(_) => Err(invalid_utf8()),
            Err(e) => Err(e),
        }
    } else {
        ret
    }
}

pub(crate) fn default_read_to_string<R: Read + ?Sized>(r: &mut R, buf: &mut String) -> Result<usize> {
    append_to_string(buf, |b| default_read_to_end(r, b))
}

pub(crate) fn default_read_exact<R: Read + ?Sized>(r: &mut R, mut buf: &mut [u8]) -> Result<()> {
    while !buf.is_empty() {
        match r.read(buf) {
            Ok(0) => break,
            Ok(n) => {
                let tmp = buf;
                buf = &mut tmp[n..];
            }
            Err(e) => {
                if !is_interrupted(&e) {
                    return Err(e);
                }
            }
        }
    }
    if !buf.is_empty() {
        Err(Error::new_const(ErrorKind::UnexpectedEof, "failed to fill whole buffer"))
    } else {
        Ok(())
    }
}

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        default_read_to_end(self, buf)
    }
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        default_read_to_string(self, buf)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        default_read_exact(self, buf)
    }
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
    fn bytes(self) -> Bytes<Self>
    where
        Self: Sized,
    {
        Bytes { inner: self }
    }
    fn chain<R: Read>(self, next: R) -> Chain<Self, R>
    where
        Self: Sized,
    {
        Chain { first: self, second: next, done_first: false }
    }
    fn take(self, limit: u64) -> Take<Self>
    where
        Self: Sized,
    {
        Take { inner: self, limit }
    }
}

pub fn read_to_string<R: Read>(mut reader: R) -> Result<String> {
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    Ok(buf)
}

// ---------------------------------------------------------------------------
// Write

struct Adapter<'a, T: ?Sized + 'a> {
    inner: &'a mut T,
    error: Result<()>,
}

impl<'a, T: Write + ?Sized> fmt::Write for Adapter<'a, T> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self.inner.write_all(s.as_bytes()) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.error = Err(e);
                Err(fmt::Error)
            }
        }
    }
}

pub(crate) fn default_write_fmt<W: Write + ?Sized>(w: &mut W, args: fmt::Arguments<'_>) -> Result<()> {
    let mut output = Adapter { inner: w, error: Ok(()) };
    match fmt::write(&mut output, args) {
        Ok(()) => Ok(()),
        Err(_) => {
            if output.error.is_err() {
                output.error
            } else {
                Err(Error::new_const(ErrorKind::Uncategorized, "formatter error"))
            }
        }
    }
}

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;

    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => return Err(Error::new_const(ErrorKind::WriteZero, "failed to write whole buffer")),
                Ok(n) => buf = &buf[n..],
                Err(e) => {
                    if !is_interrupted(&e) {
                        return Err(e);
                    }
                }
            }
        }
        Ok(())
    }
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        default_write_fmt(self, args)
    }
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

// ---------------------------------------------------------------------------
// Seek

#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

pub trait Seek {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;

    fn rewind(&mut self) -> Result<()> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }
    fn stream_position(&mut self) -> Result<u64> {
        self.seek(SeekFrom::Current(0))
    }
    fn seek_relative(&mut self, offset: i64) -> Result<()> {
        self.seek(SeekFrom::Current(offset))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// BufRead

pub(crate) fn read_until<R: BufRead + ?Sized>(r: &mut R, delim: u8, buf: &mut Vec<u8>) -> Result<usize> {
    let mut read = 0;
    loop {
        let mut done = false;
        let mut used = 0;
        match r.fill_buf() {
            Ok(available) => {
                let mut i = 0;
                while i < available.len() && available[i] != delim {
                    i += 1;
                }
                if i < available.len() {
                    buf.extend_from_slice(&available[..i + 1]);
                    done = true;
                    used = i + 1;
                } else {
                    buf.extend_from_slice(available);
                    used = available.len();
                }
            }
            Err(e) => {
                if !is_interrupted(&e) {
                    return Err(e);
                }
                continue;
            }
        }
        r.consume(used);
        read += used;
        if done || used == 0 {
            return Ok(read);
        }
    }
}

pub trait BufRead: Read {
    fn fill_buf(&mut self) -> Result<&[u8]>;
    fn consume(&mut self, amt: usize);

    fn has_data_left(&mut self) -> Result<bool> {
        match self.fill_buf() {
            Ok(b) => Ok(!b.is_empty()),
            Err(e) => Err(e),
        }
    }
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        read_until(self, byte, buf)
    }
    fn skip_until(&mut self, byte: u8) -> Result<usize> {
        let mut v = Vec::new();
        read_until(self, byte, &mut v)
    }
    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        append_to_string(buf, |b| read_until(self, b'\n', b))
    }
    fn split(self, byte: u8) -> Split<Self>
    where
        Self: Sized,
    {
        Split { buf: self, delim: byte }
    }
    fn lines(self) -> Lines<Self>
    where
        Self: Sized,
    {
        Lines { buf: self }
    }
}

#[derive(Debug)]
pub struct Split<B> {
    buf: B,
    delim: u8,
}

impl<B: BufRead> Iterator for Split<B> {
    type Item = Result<Vec<u8>>;
    fn next(&mut self) -> Option<Result<Vec<u8>>> {
        let mut buf = Vec::new();
        match read_until(&mut self.buf, self.delim, &mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                if buf[buf.len() - 1] == self.delim {
                    buf.pop();
                }
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

#[derive(Debug)]
pub struct Lines<B> {
    buf: B,
}

impl<B: BufRead> Iterator for Lines<B> {
    type Item = Result<String>;
    fn next(&mut self) -> Option<Result<String>> {
        let mut buf = String::new();
        match self.buf.read_line(&mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                if buf.ends_with('\n') {
                    buf.pop();
                    if buf.ends_with('\r') {
                        buf.pop();
                    }
                }
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

// ---------------------------------------------------------------------------
// adapters

#[derive(Debug)]
pub struct Bytes<R> {
    inner: R,
}

impl<R: Read> Iterator for Bytes<R> {
    type Item = Result<u8>;
    fn next(&mut self) -> Option<Result<u8>> {
        let mut byte = [0u8; 1];
        loop {
            match self.inner.read(&mut byte) {
                Ok(0) => return None,
                Ok(_) => return Some(Ok(byte[0])),
                Err(e) => {
                    if !is_interrupted(&e) {
                        return Some(Err(e));
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Chain<T, U> {
    first: T,
    second: U,
    done_first: bool,
}

impl<T, U> Chain<T, U> {
    pub fn into_inner(self) -> (T, U) {
        (self.first, self.second)
    }
    pub fn get_ref(&self) -> (&T, &U) {
        (&self.first, &self.second)
    }
    pub fn get_mut(&mut self) -> (&mut T, &mut U) {
        (&mut self.first, &mut self.second)
    }
}

impl<T: Read, U: Read> Read for Chain<T, U> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.done_first {
            match self.first.read(buf)? {
                0 if !buf.is_empty() => self.done_first = true,
                n => return Ok(n),
            }
        }
        self.second.read(buf)
    }
}

impl<T: BufRead, U: BufRead> BufRead for Chain<T, U> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if !self.done_first {
            let empty = self.first.fill_buf()?.is_empty();
            if !empty {
                return self.first.fill_buf();
            }
            self.done_first = true;
        }
        self.second.fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        if !self.done_first { self.first.consume(amt) } else { self.second.consume(amt) }
    }
}

#[derive(Debug)]
pub struct Take<T> {
    inner: T,
    limit: u64,
}

impl<T> Take<T> {
    pub fn limit(&self) -> u64 {
        self.limit
    }
    pub fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
    }
    pub fn into_inner(self) -> T {
        self.inner
    }
    pub fn get_ref(&self) -> &T {
        &self.inner
    }
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: Read> Read for Take<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.limit == 0 {
            return Ok(0);
        }
        let max = cmp::min(buf.len() as u64, self.limit) as usize;
        let n = self.inner.read(&mut buf[..max])?;
        self.limit -= n as u64;
        Ok(n)
    }
}

impl<T: BufRead> BufRead for Take<T> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.limit == 0 {
            return Ok(&[]);
        }
        let limit = self.limit;
        let buf = self.inner.fill_buf()?;
        let cap = cmp::min(buf.len() as u64, limit) as usize;
        Ok(&buf[..cap])
    }
    fn consume(&mut self, amt: usize) {
        let amt = cmp::min(amt as u64, self.limit) as usize;
        self.limit -= amt as u64;
        self.inner.consume(amt);
    }
}

pub(crate) fn raw_write_all(fd: i32, mut buf: &[u8]) -> Result<()> {
    while !buf.is_empty() {
        let n = sys::cvt(unsafe { sys::rcc_os_write(fd, buf.as_ptr(), buf.len()) })? as usize;
        if n == 0 {
            return Err(Error::new_const(ErrorKind::WriteZero, "failed to write whole buffer"));
        }
        buf = &buf[n..];
    }
    Ok(())
}
