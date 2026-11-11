//! `std::os::unix`.

pub mod ffi;
pub mod fs;
pub mod process;

pub mod io {
    pub use crate::os::fd::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
}

pub mod prelude {
    pub use super::ffi::{OsStrExt, OsStringExt};
    pub use super::fs::{DirEntryExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
    pub use super::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
    pub use super::process::{CommandExt, ExitStatusExt};
}
