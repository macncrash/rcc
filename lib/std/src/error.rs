//! `std::error::Error` and the `Box<dyn Error>` conversions.
//! (rcc drops auto traits from trait objects, so `dyn Error + Send + Sync`
//! is the same type as `dyn Error`: one set of impls covers both.)

use crate::borrow::Cow;
use crate::boxed::Box;
use crate::fmt::{self, Debug, Display};
use crate::string::String;

pub trait Error: Debug + Display {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }
    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl<'a, E: Error + 'a> From<E> for Box<dyn Error + 'a> {
    fn from(err: E) -> Box<dyn Error + 'a> {
        Box::new(err)
    }
}

struct StringError(String);

impl Error for StringError {
    fn description(&self) -> &str {
        &self.0
    }
}
impl Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}
impl Debug for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<'a> From<String> for Box<dyn Error + 'a> {
    fn from(err: String) -> Box<dyn Error + 'a> {
        Box::new(StringError(err))
    }
}
impl<'a, 'b> From<&'b str> for Box<dyn Error + 'a> {
    fn from(err: &'b str) -> Box<dyn Error + 'a> {
        Box::new(StringError(String::from(err)))
    }
}
impl<'a, 'b> From<Cow<'b, str>> for Box<dyn Error + 'a> {
    fn from(err: Cow<'b, str>) -> Box<dyn Error + 'a> {
        Box::new(StringError(err.into_owned()))
    }
}

impl<T: Error> Error for Box<T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        (**self).source()
    }
}
impl<'a, T: Error + ?Sized> Error for &'a T {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        (**self).source()
    }
}

impl Error for fmt::Error {}
impl Error for crate::num::ParseIntError {}
impl Error for crate::num::ParseFloatError {}
impl Error for crate::num::TryFromIntError {}
impl Error for crate::str::Utf8Error {}
impl Error for crate::str::ParseBoolError {}
impl Error for crate::str::ParseCharError {}
impl Error for crate::string::FromUtf8Error {}
impl Error for crate::array::TryFromSliceError {}
impl Error for crate::cell::BorrowError {}
impl Error for crate::cell::BorrowMutError {}
impl Error for crate::convert::Infallible {}
