//! Runtime entry points the compiler and the macro expander rely on.

use crate::fmt;
use crate::fmt::Write;

extern "C" {
    fn rcc_exit(code: i32) -> !;
}

/// `panic!` lowers to a call of this function.
/// On the main thread it prints the message and exits with 101; on a
/// spawned thread the payload goes to `join()` (see panic.rs).
pub fn panic_fmt(args: fmt::Arguments<'_>, file: &'static str, line: u32, col: u32) -> ! {
    crate::panic::begin_panic_fmt(args, file, line, col)
}

/// `assert_eq!` / `assert_ne!` failure.
pub fn assert_failed(
    op: &'static str,
    left: &dyn fmt::Debug,
    right: &dyn fmt::Debug,
    args: Option<fmt::Arguments<'_>>,
    file: &'static str,
    line: u32,
    col: u32,
) -> ! {
    match args {
        Some(a) => panic_fmt(
            format_args!("assertion `left {} right` failed: {}\n  left: {:?}\n right: {:?}", op, a, left, right),
            file,
            line,
            col,
        ),
        None => panic_fmt(format_args!("assertion `left {} right` failed\n  left: {:?}\n right: {:?}", op, left, right), file, line, col),
    }
}

/// What `fn main` may return.
pub trait Termination {
    fn report(self) -> i32;
}

impl Termination for () {
    fn report(self) -> i32 {
        0
    }
}

impl Termination for ! {
    fn report(self) -> i32 {
        self
    }
}

impl<T: Termination, E: fmt::Debug> Termination for Result<T, E> {
    fn report(self) -> i32 {
        match self {
            Ok(v) => v.report(),
            Err(e) => {
                let mut err = crate::io::RawFd { fd: 2 };
                let _ = fmt::write(&mut err, format_args!("Error: {:?}\n", e));
                1
            }
        }
    }
}

/// The C `main` calls this with the user's `main`.
#[lang = "start"]
fn lang_start<T: Termination>(main: fn() -> T) -> i32 {
    main().report()
}

/// Comparison used for string-literal patterns.
#[lang = "str_eq"]
fn str_eq(a: &str, b: &str) -> bool {
    a == b
}

/// Comparison used for byte-string-literal patterns.
#[lang = "slice_eq"]
fn slice_eq<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    a == b
}
