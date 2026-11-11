//! `io::Error` / `io::ErrorKind` with real std's Display/Debug text.

use crate::boxed::Box;
use crate::error;
use crate::fmt;
use crate::os::sys;
use crate::string::String;

pub type Result<T> = crate::result::Result<T, Error>;

/// Variant order is shared with rt.c (`RK_*` in the rcc_os_* block).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    HostUnreachable,
    NetworkUnreachable,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    NetworkDown,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    ReadOnlyFilesystem,
    FilesystemLoop,
    StaleNetworkFileHandle,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    StorageFull,
    NotSeekable,
    QuotaExceeded,
    FileTooLarge,
    ResourceBusy,
    ExecutableFileBusy,
    Deadlock,
    CrossesDevices,
    TooManyLinks,
    InvalidFilename,
    ArgumentListTooLong,
    Interrupted,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    InProgress,
    Other,
    Uncategorized,
}

impl ErrorKind {
    fn from_index(i: i32) -> ErrorKind {
        use self::ErrorKind::*;
        let all = [
            NotFound, PermissionDenied, ConnectionRefused, ConnectionReset, HostUnreachable,
            NetworkUnreachable, ConnectionAborted, NotConnected, AddrInUse, AddrNotAvailable,
            NetworkDown, BrokenPipe, AlreadyExists, WouldBlock, NotADirectory, IsADirectory,
            DirectoryNotEmpty, ReadOnlyFilesystem, FilesystemLoop, StaleNetworkFileHandle,
            InvalidInput, InvalidData, TimedOut, WriteZero, StorageFull, NotSeekable,
            QuotaExceeded, FileTooLarge, ResourceBusy, ExecutableFileBusy, Deadlock,
            CrossesDevices, TooManyLinks, InvalidFilename, ArgumentListTooLong, Interrupted,
            Unsupported, UnexpectedEof, OutOfMemory, InProgress, Other, Uncategorized,
        ];
        if i >= 0 && (i as usize) < all.len() { all[i as usize] } else { Uncategorized }
    }

    pub(crate) fn as_str(&self) -> &'static str {
        use self::ErrorKind::*;
        match *self {
            NotFound => "entity not found",
            PermissionDenied => "permission denied",
            ConnectionRefused => "connection refused",
            ConnectionReset => "connection reset",
            HostUnreachable => "host unreachable",
            NetworkUnreachable => "network unreachable",
            ConnectionAborted => "connection aborted",
            NotConnected => "not connected",
            AddrInUse => "address in use",
            AddrNotAvailable => "address not available",
            NetworkDown => "network down",
            BrokenPipe => "broken pipe",
            AlreadyExists => "entity already exists",
            WouldBlock => "operation would block",
            NotADirectory => "not a directory",
            IsADirectory => "is a directory",
            DirectoryNotEmpty => "directory not empty",
            ReadOnlyFilesystem => "read-only filesystem or storage medium",
            FilesystemLoop => "filesystem loop or indirection limit (e.g. symlink loop)",
            StaleNetworkFileHandle => "stale network file handle",
            InvalidInput => "invalid input parameter",
            InvalidData => "invalid data",
            TimedOut => "timed out",
            WriteZero => "write zero",
            StorageFull => "no storage space",
            NotSeekable => "seek on unseekable file",
            QuotaExceeded => "quota exceeded",
            FileTooLarge => "file too large",
            ResourceBusy => "resource busy",
            ExecutableFileBusy => "executable file busy",
            Deadlock => "deadlock",
            CrossesDevices => "cross-device link or rename",
            TooManyLinks => "too many links",
            InvalidFilename => "invalid filename",
            ArgumentListTooLong => "argument list too long",
            Interrupted => "operation interrupted",
            Unsupported => "unsupported",
            UnexpectedEof => "unexpected end of file",
            OutOfMemory => "out of memory",
            InProgress => "in progress",
            Other => "other error",
            Uncategorized => "uncategorized error",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub(crate) fn decode_error_kind(code: i32) -> ErrorKind {
    ErrorKind::from_index(unsafe { sys::rcc_os_error_kind(code) })
}

pub(crate) fn os_error_string(code: i32) -> String {
    let mut buf = [0u8; 256];
    let n = unsafe { sys::rcc_os_strerror(code, buf.as_mut_ptr(), buf.len()) };
    String::from_utf8_lossy(&buf[..n]).into_owned()
}

struct Custom {
    kind: ErrorKind,
    error: Box<dyn error::Error + Send + Sync>,
}

enum Repr {
    Os(i32),
    Simple(ErrorKind),
    SimpleMessage(ErrorKind, &'static str),
    Custom(Box<Custom>),
}

pub struct Error {
    repr: Repr,
}

impl Error {
    pub fn new<E: Into<Box<dyn error::Error + Send + Sync>>>(kind: ErrorKind, error: E) -> Error {
        Error { repr: Repr::Custom(Box::new(Custom { kind, error: error.into() })) }
    }
    pub fn other<E: Into<Box<dyn error::Error + Send + Sync>>>(error: E) -> Error {
        Error::new(ErrorKind::Other, error)
    }
    pub(crate) fn new_const(kind: ErrorKind, msg: &'static str) -> Error {
        Error { repr: Repr::SimpleMessage(kind, msg) }
    }
    pub fn last_os_error() -> Error {
        // rt.c reports errors by return value; errno is not kept.
        Error::from_raw_os_error(0)
    }
    pub fn from_raw_os_error(code: i32) -> Error {
        Error { repr: Repr::Os(code) }
    }
    pub fn raw_os_error(&self) -> Option<i32> {
        match &self.repr {
            Repr::Os(c) => Some(*c),
            _ => None,
        }
    }
    pub fn get_ref(&self) -> Option<&(dyn error::Error + Send + Sync + 'static)> {
        match &self.repr {
            Repr::Custom(c) => Some(&*c.error),
            _ => None,
        }
    }
    pub fn get_mut(&mut self) -> Option<&mut (dyn error::Error + Send + Sync + 'static)> {
        match &mut self.repr {
            Repr::Custom(c) => Some(&mut *c.error),
            _ => None,
        }
    }
    pub fn into_inner(self) -> Option<Box<dyn error::Error + Send + Sync>> {
        match self.repr {
            Repr::Custom(c) => {
                let c = *c;
                Some(c.error)
            }
            _ => None,
        }
    }
    pub fn kind(&self) -> ErrorKind {
        match &self.repr {
            Repr::Os(c) => decode_error_kind(*c),
            Repr::Simple(k) => *k,
            Repr::SimpleMessage(k, _) => *k,
            Repr::Custom(c) => c.kind,
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error { repr: Repr::Simple(kind) }
    }
}

impl From<crate::string::FromUtf8Error> for Error {
    fn from(_: crate::string::FromUtf8Error) -> Error {
        Error::new_const(ErrorKind::InvalidData, "stream did not contain valid UTF-8")
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Os(code) => {
                let kind = decode_error_kind(*code);
                let message = os_error_string(*code);
                f.debug_struct("Os").field("code", code).field("kind", &kind).field("message", &message).finish()
            }
            Repr::Custom(c) => f.debug_struct("Custom").field("kind", &c.kind).field("error", &c.error).finish(),
            Repr::Simple(kind) => f.debug_tuple("Kind").field(kind).finish(),
            Repr::SimpleMessage(kind, message) => {
                f.debug_struct("Error").field("kind", kind).field("message", message).finish()
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Os(code) => {
                let detail = os_error_string(*code);
                write!(f, "{} (os error {})", detail, code)
            }
            Repr::Custom(c) => fmt::Display::fmt(&*c.error, f),
            Repr::Simple(kind) => f.write_str(kind.as_str()),
            Repr::SimpleMessage(_, msg) => f.write_str(msg),
        }
    }
}

impl error::Error for Error {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        match &self.repr {
            Repr::Os(_) | Repr::Simple(_) => self.kind().as_str(),
            Repr::SimpleMessage(_, msg) => msg,
            Repr::Custom(c) => c.error.description(),
        }
    }
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.repr {
            Repr::Custom(c) => c.error.source(),
            _ => None,
        }
    }
}
