//! stdin / stdout / stderr. The buffers live in rt.c (one process-wide
//! stdin buffer shared by every `Stdin`/`StdinLock`; stdout is C stdio with
//! LineWriter semantics; stderr unbuffered). Locks are plain handles: the C
//! side serialises each call.

use super::{BufRead, Lines, Read, Result, Write};
use crate::fmt;
use crate::os::sys;
use crate::string::String;
use crate::vec::Vec;

pub trait IsTerminal {
    fn is_terminal(&self) -> bool;
}

fn isatty(fd: i32) -> bool {
    unsafe { sys::rcc_os_isatty(fd) != 0 }
}

// ---------------------------------------------------------------------------
// stdin

pub struct Stdin {
    _priv: (),
}
pub struct StdinLock<'a> {
    _marker: crate::marker::PhantomData<&'a ()>,
}

pub fn stdin() -> Stdin {
    Stdin { _priv: () }
}

impl Stdin {
    pub fn lock(&self) -> StdinLock<'static> {
        StdinLock { _marker: crate::marker::PhantomData }
    }
    pub fn read_line(&self, buf: &mut String) -> Result<usize> {
        self.lock().read_line(buf)
    }
    pub fn lines(self) -> Lines<StdinLock<'static>> {
        self.lock().lines()
    }
}

fn stdin_read(buf: &mut [u8]) -> Result<usize> {
    Ok(sys::cvt(unsafe { sys::rcc_os_stdin_read(buf.as_mut_ptr(), buf.len()) })? as usize)
}

impl Read for Stdin {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        stdin_read(buf)
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.lock().read_to_end(buf)
    }
}

impl<'a> Read for StdinLock<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        stdin_read(buf)
    }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        // drain what is buffered, then read the rest directly
        let mut n = 0;
        loop {
            let avail = {
                let b = self.fill_buf()?;
                buf.extend_from_slice(b);
                b.len()
            };
            if avail == 0 {
                return Ok(n);
            }
            self.consume(avail);
            n += avail;
        }
    }
}

impl<'a> BufRead for StdinLock<'a> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        let mut p: *const u8 = crate::ptr::null();
        let n = sys::cvt(unsafe { sys::rcc_os_stdin_fill(&mut p) })? as usize;
        Ok(unsafe { crate::slice::from_raw_parts(p, n) })
    }
    fn consume(&mut self, amt: usize) {
        unsafe { sys::rcc_os_stdin_consume(amt) }
    }
}

impl IsTerminal for Stdin {
    fn is_terminal(&self) -> bool {
        isatty(0)
    }
}
impl<'a> IsTerminal for StdinLock<'a> {
    fn is_terminal(&self) -> bool {
        isatty(0)
    }
}
impl fmt::Debug for Stdin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Stdin { .. }")
    }
}
impl<'a> fmt::Debug for StdinLock<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StdinLock { .. }")
    }
}

// ---------------------------------------------------------------------------
// stdout

pub struct Stdout {
    _priv: (),
}
pub struct StdoutLock<'a> {
    _marker: crate::marker::PhantomData<&'a ()>,
}

pub fn stdout() -> Stdout {
    Stdout { _priv: () }
}

impl Stdout {
    pub fn lock(&self) -> StdoutLock<'static> {
        StdoutLock { _marker: crate::marker::PhantomData }
    }
}

fn stdout_write(buf: &[u8]) -> Result<usize> {
    let r = unsafe { sys::rcc_os_stdout_write(buf.as_ptr(), buf.len()) };
    if r != 0 {
        return Err(super::Error::from_raw_os_error(r));
    }
    Ok(buf.len())
}

fn stdout_flush() -> Result<()> {
    let r = unsafe { sys::rcc_os_stdout_flush() };
    if r != 0 { Err(super::Error::from_raw_os_error(r)) } else { Ok(()) }
}

impl Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        stdout_write(buf)
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        stdout_write(buf)?;
        Ok(())
    }
    fn flush(&mut self) -> Result<()> {
        stdout_flush()
    }
}
impl<'b> Write for &'b Stdout {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        stdout_write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        stdout_flush()
    }
}
impl<'a> Write for StdoutLock<'a> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        stdout_write(buf)
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        stdout_write(buf)?;
        Ok(())
    }
    fn flush(&mut self) -> Result<()> {
        stdout_flush()
    }
}
impl IsTerminal for Stdout {
    fn is_terminal(&self) -> bool {
        isatty(1)
    }
}
impl<'a> IsTerminal for StdoutLock<'a> {
    fn is_terminal(&self) -> bool {
        isatty(1)
    }
}
impl fmt::Debug for Stdout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Stdout { .. }")
    }
}
impl<'a> fmt::Debug for StdoutLock<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StdoutLock { .. }")
    }
}

// ---------------------------------------------------------------------------
// stderr

pub struct Stderr {
    _priv: (),
}
pub struct StderrLock<'a> {
    _marker: crate::marker::PhantomData<&'a ()>,
}

pub fn stderr() -> Stderr {
    Stderr { _priv: () }
}

impl Stderr {
    pub fn lock(&self) -> StderrLock<'static> {
        StderrLock { _marker: crate::marker::PhantomData }
    }
}

fn stderr_write(buf: &[u8]) -> Result<usize> {
    // like real std, a closed stderr (EBADF) swallows output
    match sys::cvt(unsafe { sys::rcc_os_write(2, buf.as_ptr(), buf.len()) }) {
        Ok(n) => Ok(n as usize),
        Err(e) => {
            if e.raw_os_error() == Some(9) { Ok(buf.len()) } else { Err(e) }
        }
    }
}

impl Write for Stderr {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        stderr_write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
impl<'b> Write for &'b Stderr {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        stderr_write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
impl<'a> Write for StderrLock<'a> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        stderr_write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
impl IsTerminal for Stderr {
    fn is_terminal(&self) -> bool {
        isatty(2)
    }
}
impl<'a> IsTerminal for StderrLock<'a> {
    fn is_terminal(&self) -> bool {
        isatty(2)
    }
}
impl fmt::Debug for Stderr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Stderr { .. }")
    }
}
impl<'a> fmt::Debug for StderrLock<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StderrLock { .. }")
    }
}
