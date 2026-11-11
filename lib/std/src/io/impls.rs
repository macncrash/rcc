//! Read/Write/BufRead/Seek for references, boxes, byte slices and `Vec<u8>`.

use super::{BufRead, Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use crate::boxed::Box;
use crate::cmp;
use crate::fmt;
use crate::string::String;
use crate::vec::Vec;

impl<'a, R: Read + ?Sized> Read for &'a mut R {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).read(buf)
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_to_end(buf)
    }
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_to_string(buf)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        (**self).read_exact(buf)
    }
}
impl<R: Read + ?Sized> Read for Box<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).read(buf)
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_to_end(buf)
    }
    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_to_string(buf)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        (**self).read_exact(buf)
    }
}
impl<'a, W: Write + ?Sized> Write for &'a mut W {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        (**self).write_all(buf)
    }
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        (**self).write_fmt(args)
    }
}
impl<W: Write + ?Sized> Write for Box<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        (**self).write_all(buf)
    }
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        (**self).write_fmt(args)
    }
}
impl<'a, B: BufRead + ?Sized> BufRead for &'a mut B {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        (**self).fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        (**self).consume(amt)
    }
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_until(byte, buf)
    }
    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_line(buf)
    }
}
impl<B: BufRead + ?Sized> BufRead for Box<B> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        (**self).fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        (**self).consume(amt)
    }
}
impl<'a, S: Seek + ?Sized> Seek for &'a mut S {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        (**self).seek(pos)
    }
}
impl<S: Seek + ?Sized> Seek for Box<S> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        (**self).seek(pos)
    }
}

impl<'a> Read for &'a [u8] {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let amt = cmp::min(buf.len(), self.len());
        let src: &'a [u8] = *self;
        buf[..amt].copy_from_slice(&src[..amt]);
        *self = &src[amt..];
        Ok(amt)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let src: &'a [u8] = *self;
        if buf.len() > src.len() {
            *self = &src[src.len()..];
            return Err(Error::new_const(ErrorKind::UnexpectedEof, "failed to fill whole buffer"));
        }
        let n = buf.len();
        buf.copy_from_slice(&src[..n]);
        *self = &src[n..];
        Ok(())
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let src: &'a [u8] = *self;
        buf.extend_from_slice(src);
        *self = &src[src.len()..];
        Ok(src.len())
    }
}

impl<'a> BufRead for &'a [u8] {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        Ok(*self)
    }
    fn consume(&mut self, amt: usize) {
        let src: &'a [u8] = *self;
        *self = &src[amt..];
    }
}

impl<'a> Write for &'a mut [u8] {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let dst: &'a mut [u8] = crate::mem::replace(self, &mut []);
        let amt = cmp::min(data.len(), dst.len());
        let (a, b) = dst.split_at_mut(amt);
        a.copy_from_slice(&data[..amt]);
        *self = b;
        Ok(amt)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.extend_from_slice(buf);
        Ok(())
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
