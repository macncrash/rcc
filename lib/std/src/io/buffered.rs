//! `BufReader`, `BufWriter` (flushes on drop), `LineWriter`.

use super::{BufRead, Error, ErrorKind, Read, Result, Seek, SeekFrom, Write, DEFAULT_BUF_SIZE};
use crate::cmp;
use crate::fmt;
use crate::string::String;
use crate::vec::Vec;

pub struct BufReader<R> {
    buf: Vec<u8>, // len == capacity of the buffer; [pos, filled) is valid
    pos: usize,
    filled: usize,
    inner: R,
}

impl<R: Read> BufReader<R> {
    pub fn new(inner: R) -> BufReader<R> {
        BufReader::with_capacity(DEFAULT_BUF_SIZE, inner)
    }
    pub fn with_capacity(capacity: usize, inner: R) -> BufReader<R> {
        BufReader { buf: crate::vec::from_elem(0u8, capacity), pos: 0, filled: 0, inner }
    }
}

impl<R> BufReader<R> {
    pub fn get_ref(&self) -> &R {
        &self.inner
    }
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }
    pub fn buffer(&self) -> &[u8] {
        &self.buf[self.pos..self.filled]
    }
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }
    pub fn into_inner(self) -> R {
        self.inner
    }
    fn discard_buffer(&mut self) {
        self.pos = 0;
        self.filled = 0;
    }
}

impl<R: Read> Read for BufReader<R> {
    fn read(&mut self, out: &mut [u8]) -> Result<usize> {
        if self.pos == self.filled && out.len() >= self.buf.len() {
            self.discard_buffer();
            return self.inner.read(out);
        }
        let n = {
            let rem = self.fill_buf()?;
            let n = cmp::min(rem.len(), out.len());
            out[..n].copy_from_slice(&rem[..n]);
            n
        };
        self.consume(n);
        Ok(n)
    }
    fn read_to_end(&mut self, out: &mut Vec<u8>) -> Result<usize> {
        let n = self.filled - self.pos;
        out.extend_from_slice(&self.buf[self.pos..self.filled]);
        self.discard_buffer();
        Ok(n + self.inner.read_to_end(out)?)
    }
    fn read_to_string(&mut self, out: &mut String) -> Result<usize> {
        super::append_to_string(out, |b| self.read_to_end(b))
    }
}

impl<R: Read> BufRead for BufReader<R> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.pos >= self.filled {
            let n = self.inner.read(&mut self.buf)?;
            self.pos = 0;
            self.filled = n;
        }
        Ok(&self.buf[self.pos..self.filled])
    }
    fn consume(&mut self, amt: usize) {
        self.pos = cmp::min(self.pos + amt, self.filled);
    }
}

impl<R: Seek> Seek for BufReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let result;
        if let SeekFrom::Current(n) = pos {
            let remainder = (self.filled - self.pos) as i64;
            result = self.inner.seek(SeekFrom::Current(n - remainder))?;
        } else {
            result = self.inner.seek(pos)?;
        }
        self.discard_buffer();
        Ok(result)
    }
    fn stream_position(&mut self) -> Result<u64> {
        let remainder = (self.filled - self.pos) as u64;
        let p = self.inner.stream_position()?;
        Ok(p - remainder)
    }
}

impl<R: fmt::Debug> fmt::Debug for BufReader<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufReader")
            .field("reader", &self.inner)
            .field("buffer", &format_args!("{}/{}", self.filled - self.pos, self.buf.len()))
            .finish()
    }
}

// ---------------------------------------------------------------------------

pub struct BufWriter<W: Write> {
    buf: Vec<u8>,
    cap: usize,
    panicked: bool,
    inner: Option<W>,
}

#[derive(Debug)]
pub struct IntoInnerError<W>(W, Error);

impl<W> IntoInnerError<W> {
    pub fn error(&self) -> &Error {
        &self.1
    }
    pub fn into_inner(self) -> W {
        self.0
    }
    pub fn into_error(self) -> Error {
        self.1
    }
    pub fn into_parts(self) -> (Error, W) {
        (self.1, self.0)
    }
}
impl<W> fmt::Display for IntoInnerError<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.1, f)
    }
}
impl<W: fmt::Debug> crate::error::Error for IntoInnerError<W> {}
impl<W> From<IntoInnerError<W>> for Error {
    fn from(e: IntoInnerError<W>) -> Error {
        e.1
    }
}

impl<W: Write> BufWriter<W> {
    pub fn new(inner: W) -> BufWriter<W> {
        BufWriter::with_capacity(DEFAULT_BUF_SIZE, inner)
    }
    pub fn with_capacity(capacity: usize, inner: W) -> BufWriter<W> {
        BufWriter { buf: Vec::with_capacity(capacity), cap: capacity, panicked: false, inner: Some(inner) }
    }
    fn inner_mut(&mut self) -> &mut W {
        match &mut self.inner {
            Some(w) => w,
            None => unreachable!(),
        }
    }
    fn flush_buf(&mut self) -> Result<()> {
        let mut written = 0;
        let len = self.buf.len();
        let mut ret = Ok(());
        while written < len {
            self.panicked = true;
            let r = {
                let data = &self.buf[written..];
                let w = match &mut self.inner {
                    Some(w) => w,
                    None => unreachable!(),
                };
                w.write(data)
            };
            self.panicked = false;
            match r {
                Ok(0) => {
                    ret = Err(Error::new_const(ErrorKind::WriteZero, "failed to write the buffered data"));
                    break;
                }
                Ok(n) => written += n,
                Err(e) => {
                    if e.kind() != ErrorKind::Interrupted {
                        ret = Err(e);
                        break;
                    }
                }
            }
        }
        if written > 0 {
            self.buf.drain(..written);
        }
        ret
    }
    pub fn get_ref(&self) -> &W {
        match &self.inner {
            Some(w) => w,
            None => unreachable!(),
        }
    }
    pub fn get_mut(&mut self) -> &mut W {
        self.inner_mut()
    }
    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }
    pub fn capacity(&self) -> usize {
        self.cap
    }
    pub fn into_inner(mut self) -> crate::result::Result<W, IntoInnerError<BufWriter<W>>> {
        match self.flush_buf() {
            Err(e) => Err(IntoInnerError(self, e)),
            Ok(()) => Ok(self.inner.take().unwrap()),
        }
    }
    pub fn into_parts(mut self) -> (W, crate::result::Result<Vec<u8>, WriterPanicked>) {
        let buf = crate::mem::replace(&mut self.buf, Vec::new());
        let w = self.inner.take().unwrap();
        (w, Ok(buf))
    }
}

#[derive(Debug)]
pub struct WriterPanicked {
    buf: Vec<u8>,
}
impl WriterPanicked {
    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }
}

impl<W: Write> Write for BufWriter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if self.buf.len() + buf.len() > self.cap {
            self.flush_buf()?;
        }
        if buf.len() >= self.cap {
            self.panicked = true;
            let r = self.inner_mut().write(buf);
            self.panicked = false;
            r
        } else {
            self.buf.extend_from_slice(buf);
            Ok(buf.len())
        }
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        if self.buf.len() + buf.len() > self.cap {
            self.flush_buf()?;
        }
        if buf.len() >= self.cap {
            self.panicked = true;
            let r = self.inner_mut().write_all(buf);
            self.panicked = false;
            r
        } else {
            self.buf.extend_from_slice(buf);
            Ok(())
        }
    }
    fn flush(&mut self) -> Result<()> {
        self.flush_buf()?;
        self.inner_mut().flush()
    }
}

impl<W: Write + Seek> Seek for BufWriter<W> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.flush_buf()?;
        self.inner_mut().seek(pos)
    }
}

impl<W: Write> Drop for BufWriter<W> {
    fn drop(&mut self) {
        if self.inner.is_some() && !self.panicked {
            let _r = self.flush_buf();
        }
    }
}

impl<W: Write + fmt::Debug> fmt::Debug for BufWriter<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufWriter")
            .field("writer", self.get_ref())
            .field("buffer", &format_args!("{}/{}", self.buf.len(), self.cap))
            .finish()
    }
}

// ---------------------------------------------------------------------------

/// Buffers output and flushes after each complete line.
pub struct LineWriter<W: Write> {
    inner: BufWriter<W>,
}

impl<W: Write> LineWriter<W> {
    pub fn new(inner: W) -> LineWriter<W> {
        LineWriter::with_capacity(1024, inner)
    }
    pub fn with_capacity(capacity: usize, inner: W) -> LineWriter<W> {
        LineWriter { inner: BufWriter::with_capacity(capacity, inner) }
    }
    pub fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }
    pub fn get_mut(&mut self) -> &mut W {
        self.inner.get_mut()
    }
    pub fn into_inner(self) -> crate::result::Result<W, IntoInnerError<LineWriter<W>>> {
        let LineWriter { inner } = self;
        match inner.into_inner() {
            Ok(w) => Ok(w),
            Err(IntoInnerError(bw, e)) => Err(IntoInnerError(LineWriter { inner: bw }, e)),
        }
    }
}

impl<W: Write> Write for LineWriter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut last_nl = None;
        let mut i = buf.len();
        while i > 0 {
            if buf[i - 1] == b'\n' {
                last_nl = Some(i);
                break;
            }
            i -= 1;
        }
        match last_nl {
            None => self.inner.write(buf),
            Some(n) => {
                self.inner.write_all(&buf[..n])?;
                self.inner.flush_buf()?;
                if n < buf.len() {
                    self.inner.write_all(&buf[n..])?;
                }
                Ok(buf.len())
            }
        }
    }
    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

impl<W: Write + fmt::Debug> fmt::Debug for LineWriter<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineWriter").field("writer", self.get_ref()).finish()
    }
}
