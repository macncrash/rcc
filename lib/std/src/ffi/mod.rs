//! `std::ffi`: `OsStr`/`OsString` (unix: arbitrary bytes, usually UTF-8)
//! and a minimal `CStr`/`CString`.
//!
//! Design note (rcc has no unsized struct tails): `OsStr` is NOT a
//! dynamically sized newtype over `[u8]`. It is a small sized *view*
//! `{ bytes: &[u8] }`, and `&OsStr` is a thin pointer to such a view.
//! A view for a borrowed byte string (`OsStr::new("x")`, `Path::file_name`,
//! ...) is heap-allocated and intentionally leaked (16 bytes); `OsString`
//! embeds a view of its own buffer that `Deref` refreshes, so
//! `&OsString -> &OsStr` allocates nothing. See lib/std/src/path.rs for the
//! same scheme applied to `Path`/`PathBuf`.

mod os_str;
mod c_str;

pub use self::c_str::{CStr, CString, NulError};
pub use self::os_str::{OsStr, OsString};

pub use crate::os::raw::{c_char, c_int, c_long, c_uint, c_ulong, c_void};
