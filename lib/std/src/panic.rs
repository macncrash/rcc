//! Panics. There is no unwinding (DESIGN.md): a panic on the main thread
//! prints the message and exits with 101; on a spawned thread it prints the
//! message and abandons the thread, handing the payload to `join()` (see
//! thread/mod.rs). The payload is what real std would produce: a
//! `&'static str` when the message is a plain literal (`Arguments` with no
//! pieces or a single `Lit` piece, i.e. `Arguments::as_str()` is `Some`),
//! otherwise the formatted `String`.

use crate::any::Any;
use crate::fmt;
use crate::fmt::{Piece, Write};
use crate::string::String;

extern "C" {
    fn rcc_exit(code: i32) -> !;
}

/// "\nthread 'NAME' panicked" for the current thread.
fn header() -> String {
    let mut s = String::from("\nthread '");
    crate::thread::panic_thread_name(&mut |n: &str| s.push_str(n));
    s.push_str("' panicked");
    s
}

/// Write the whole message with one write so that concurrent panics do not
/// interleave.
fn emit(text: &str) {
    let mut err = crate::io::RawFd { fd: 2 };
    let _ = err.write_str(text);
}

/// `panic!` (through `rt::panic_fmt`).
pub(crate) fn begin_panic_fmt(args: fmt::Arguments<'_>, file: &'static str, line: u32, col: u32) -> ! {
    if crate::thread::on_main_thread() {
        let mut err = crate::io::RawFd { fd: 2 };
        let _ = err.write_str("\nthread 'main' panicked at ");
        let _ = err.write_str(file);
        let _ = err.write_str(":");
        let _ = fmt::write(&mut err, format_args!("{}:{}", line, col));
        let _ = err.write_str(":\n");
        let _ = fmt::write(&mut err, args);
        let _ = err.write_str("\n");
        unsafe { rcc_exit(101) }
    }
    let literal: Option<&'static str> = match args.pieces {
        [] => Some(""),
        [Piece::Lit(s)] => Some(*s),
        _ => None,
    };
    let msg = match literal {
        Some(s) => String::from(s),
        None => fmt::format(args),
    };
    let mut text = header();
    let _ = write!(text, " at {}:{}:{}:\n{}\n", file, line, col, msg);
    emit(&text);
    let payload: Box<dyn Any + Send> = match literal {
        Some(s) => Box::new(s),
        None => Box::new(msg),
    };
    crate::thread::unwind_with(payload)
}

/// Called by rt.c (`rcc_thr_c_panic`) for runtime panics raised in C
/// (division by zero, index out of bounds) on a spawned thread. `is_static`:
/// `msg` is a C string literal (payload `&'static str`); otherwise it is a
/// temporary buffer (payload `String`). Never returns.
pub(crate) fn c_panic_hook(msg: *const u8, len: usize, is_static: i32) -> i32 {
    let text: &'static str = unsafe { crate::str::from_utf8_unchecked(crate::slice::from_raw_parts(msg, len)) };
    let mut out = header();
    out.push_str(":\n");
    out.push_str(text);
    out.push_str("\n");
    emit(&out);
    let payload: Box<dyn Any + Send> = if is_static != 0 { Box::new(text) } else { Box::new(String::from(text)) };
    crate::thread::unwind_with(payload)
}

/// Panic with an arbitrary payload.
pub fn panic_any<M: 'static + Any + Send>(msg: M) -> ! {
    let mut out = header();
    out.push_str(":\nBox<dyn Any>\n");
    emit(&out);
    if crate::thread::on_main_thread() {
        unsafe { rcc_exit(101) }
    }
    crate::thread::unwind_with(Box::new(msg))
}
