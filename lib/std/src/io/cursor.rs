//! `io::Cursor`.

use super::{BufRead, Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use crate::boxed::Box;
use crate::cmp;
use crate::vec::Vec;

#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct Cursor<T> {
    inner: T,
    pos: u64,
}

impl<T> Cursor<T> {
    pub const fn new(inner: T) -> Cursor<T> {
        Cursor { pos: 0, inner }
    }
    pub fn into_inner(self) -> T {
        self.inner
    }
    pub const fn get_ref(&self) -> &T {
        &self.inner
    }
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
    pub const fn position(&self) -> u64 {
        self.pos
    }
    pub fn set_position(&mut self, pos: u64) {
        self.pos = pos;
    }
}

impl<T: AsRef<[u8]>> Cursor<T> {
    fn remaining(&self) -> &[u8] {
        let s = self.inner.as_ref();
        let start = cmp::min(self.pos, s.len() as u64) as usize;
        &s[start..]
    }
    pub fn remaining_slice(&self) -> &[u8] {
        self.remaining()
    }
    pub fn is_empty(&self) -> bool {
        self.pos >= self.inner.as_ref().len() as u64
    }
}

impl<T: AsRef<[u8]>> Seek for Cursor<T> {
    fn seek(&mut self, style: SeekFrom) -> Result<u64> {
        let (base, offset) = match style {
            SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            SeekFrom::End(n) => (self.inner.as_ref().len() as u64, n),
            SeekFrom::Current(n) => (self.pos, n),
        };
        let new = if offset >= 0 { base.checked_add(offset as u64) } else { base.checked_sub(offset.wrapping_neg() as u64) };
        match new {
            Some(n) => {
                self.pos = n;
                Ok(n)
            }
            None => Err(Error::new_const(
                ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            )),
        }
    }
    fn stream_position(&mut self) -> Result<u64> {
        Ok(self.pos)
    }
}

impl<T: AsRef<[u8]>> Read for Cursor<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = {
            let rem = self.remaining();
            let n = cmp::min(rem.len(), buf.len());
            buf[..n].copy_from_slice(&rem[..n]);
            n
        };
        self.pos += n as u64;
        Ok(n)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let n = buf.len();
        let ok = {
            let rem = self.remaining();
            if rem.len() < n {
                false
            } else {
                buf.copy_from_slice(&rem[..n]);
                true
            }
        };
        if !ok {
            self.pos = self.inner.as_ref().len() as u64;
            return Err(Error::new_const(ErrorKind::UnexpectedEof, "failed to fill whole buffer"));
        }
        self.pos += n as u64;
        Ok(())
    }
}

impl<T: AsRef<[u8]>> BufRead for Cursor<T> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        Ok(self.remaining())
    }
    fn consume(&mut self, amt: usize) {
        self.pos += amt as u64;
    }
}

fn slice_write(pos_mut: &mut u64, slice: &mut [u8], buf: &[u8]) -> Result<usize> {
    let pos = cmp::min(*pos_mut, slice.len() as u64) as usize;
    let amt = cmp::min(slice.len() - pos, buf.len());
    slice[pos..pos + amt].copy_from_slice(&buf[..amt]);
    *pos_mut += amt as u64;
    Ok(amt)
}

fn vec_write(pos_mut: &mut u64, vec: &mut Vec<u8>, buf: &[u8]) -> Result<usize> {
    let pos = *pos_mut as usize;
    if vec.len() < pos {
        vec.resize(pos, 0);
    }
    let overlap = cmp::min(vec.len() - pos, buf.len());
    vec[pos..pos + overlap].copy_from_slice(&buf[..overlap]);
    vec.extend_from_slice(&buf[overlap..]);
    *pos_mut = (pos + buf.len()) as u64;
    Ok(buf.len())
}

impl<'a> Write for Cursor<&'a mut [u8]> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        slice_write(&mut self.pos, self.inner, buf)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
impl<'a> Write for Cursor<&'a mut Vec<u8>> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        vec_write(&mut self.pos, self.inner, buf)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
impl Write for Cursor<Vec<u8>> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        vec_write(&mut self.pos, &mut self.inner, buf)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
impl Write for Cursor<Box<[u8]>> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        slice_write(&mut self.pos, &mut self.inner, buf)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
impl<const N: usize> Write for Cursor<[u8; N]> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        slice_write(&mut self.pos, &mut self.inner, buf)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
