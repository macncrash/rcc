//! Inherent methods of the primitive numeric types.
//!
//! Plain arithmetic operators already wrap (release semantics), so
//! `wrapping_*` are the operators themselves; the `checked_*`,
//! `overflowing_*` and `saturating_*` families are built on the
//! `*_with_overflow` intrinsics.

mod int;
pub(crate) mod float;
mod parse;

pub use self::parse::{IntErrorKind, ParseFloatError, ParseIntError};
pub(crate) use self::parse::float_error;

/// Error of the checked integer conversions (`TryFrom`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TryFromIntError(());

impl crate::fmt::Display for TryFromIntError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("out of range integral type conversion attempted")
    }
}

pub(crate) fn try_from_int_error() -> TryFromIntError {
    TryFromIntError(())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FpCategory {
    Nan,
    Infinite,
    Zero,
    Subnormal,
    Normal,
}

/// Wrapping arithmetic wrapper.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Wrapping<T>(pub T);

// Debug and Display are transparent, as in real std.
impl<T: crate::fmt::Debug> crate::fmt::Debug for Wrapping<T> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        self.0.fmt(f)
    }
}
impl<T: crate::fmt::Display> crate::fmt::Display for Wrapping<T> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        self.0.fmt(f)
    }
}
