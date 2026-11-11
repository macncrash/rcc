//! `io::copy`, `empty`, `sink`, `repeat`.

use super::{BufRead, ErrorKind, Read, Result, Write, DEFAULT_BUF_SIZE};
use crate::fmt;

pub fn copy<R: Read + ?Sized, W: Write + ?Sized>(reader: &mut R, writer: &mut W) -> Result<u64> {
    let mut buf = crate::vec::from_elem(0u8, DEFAULT_BUF_SIZE);
    let mut written = 0u64;
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => return Ok(written),
            Ok(n) => n,
            Err(e) => {
                if e.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(e);
            }
        };
        writer.write_all(&buf[..n])?;
        written += n as u64;
    }
}

#[derive(Clone, Copy, Default)]
pub struct Empty;

pub const fn empty() -> Empty {
    Empty
}

impl Read for Empty {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
        Ok(0)
    }
}
impl BufRead for Empty {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        Ok(&[])
    }
    fn consume(&mut self, _amt: usize) {}
}
impl Write for Empty {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
impl fmt::Debug for Empty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Empty")
    }
}

#[derive(Clone, Copy, Default)]
pub struct Sink;

pub const fn sink() -> Sink {
    Sink
}

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
impl fmt::Debug for Sink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Sink")
    }
}

pub struct Repeat {
    byte: u8,
}

pub const fn repeat(byte: u8) -> Repeat {
    Repeat { byte }
}

impl Read for Repeat {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        for b in buf.iter_mut() {
            *b = self.byte;
        }
        Ok(buf.len())
    }
}
impl fmt::Debug for Repeat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Repeat { .. }")
    }
}
