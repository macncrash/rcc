//! `std::os::fd`: raw file descriptors.

use crate::fs::File;

pub type RawFd = i32;

pub trait AsRawFd {
    fn as_raw_fd(&self) -> RawFd;
}
pub trait FromRawFd {
    unsafe fn from_raw_fd(fd: RawFd) -> Self;
}
pub trait IntoRawFd {
    fn into_raw_fd(self) -> RawFd;
}

impl AsRawFd for File {
    fn as_raw_fd(&self) -> RawFd {
        self.raw()
    }
}
impl FromRawFd for File {
    unsafe fn from_raw_fd(fd: RawFd) -> File {
        File::from_raw(fd)
    }
}
impl IntoRawFd for File {
    fn into_raw_fd(self) -> RawFd {
        self.into_raw()
    }
}
impl AsRawFd for crate::io::Stdin {
    fn as_raw_fd(&self) -> RawFd {
        0
    }
}
impl AsRawFd for crate::io::Stdout {
    fn as_raw_fd(&self) -> RawFd {
        1
    }
}
impl AsRawFd for crate::io::Stderr {
    fn as_raw_fd(&self) -> RawFd {
        2
    }
}
impl AsRawFd for crate::process::ChildStdin {
    fn as_raw_fd(&self) -> RawFd {
        self.raw()
    }
}
impl AsRawFd for crate::process::ChildStdout {
    fn as_raw_fd(&self) -> RawFd {
        self.raw()
    }
}
impl AsRawFd for crate::process::ChildStderr {
    fn as_raw_fd(&self) -> RawFd {
        self.raw()
    }
}
