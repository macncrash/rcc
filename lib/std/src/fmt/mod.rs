//! Formatting. The data model (`Arguments`, `Piece`, `Argument`, `Spec`,
//! `Align`) is the contract with the macro expander (see DESIGN.md and the
//! "Lowering shapes" section of docs/STATUS.md): `format_args!` expands to a
//! literal `Arguments { pieces: &[...] }`.

mod builders;
mod float;
mod num;

pub use self::builders::{DebugList, DebugMap, DebugSet, DebugStruct, DebugTuple};

use crate::string::String;

#[derive(Clone, Copy, PartialEq, Eq, Default, Hash, PartialOrd, Ord)]
pub struct Error;

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("Error")
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("an error occurred when formatting an argument")
    }
}

pub type Result = crate::result::Result<(), Error>;

pub trait Write {
    fn write_str(&mut self, s: &str) -> Result;
    fn write_char(&mut self, c: char) -> Result {
        let mut buf = [0u8; 4];
        self.write_str(c.encode_utf8(&mut buf))
    }
    fn write_fmt(&mut self, args: Arguments<'_>) -> Result {
        // `&mut Self: Write` (below) is sized, so it can become a trait object.
        let mut this = self;
        write(&mut this, args)
    }
}

impl<W: Write + ?Sized> Write for &mut W {
    fn write_str(&mut self, s: &str) -> Result {
        (**self).write_str(s)
    }
    fn write_char(&mut self, c: char) -> Result {
        (**self).write_char(c)
    }
    fn write_fmt(&mut self, args: Arguments<'_>) -> Result {
        (**self).write_fmt(args)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Unknown,
    Left,
    Center,
    Right,
}

pub type Alignment = Align;

#[derive(Clone, Copy)]
pub struct Spec {
    pub fill: char,
    pub align: Align,
    pub sign_plus: bool,
    pub sign_minus: bool,
    pub alternate: bool,
    pub zero_pad: bool,
    pub width: Option<usize>,
    pub precision: Option<usize>,
}

const DEFAULT_SPEC: Spec = Spec { fill: ' ', align: Align::Unknown, sign_plus: false, sign_minus: false, alternate: false, zero_pad: false, width: None, precision: None };

pub enum Argument<'a> {
    Display(&'a dyn Display),
    Debug(&'a dyn Debug),
    LowerHex(&'a dyn LowerHex),
    UpperHex(&'a dyn UpperHex),
    Octal(&'a dyn Octal),
    Binary(&'a dyn Binary),
    LowerExp(&'a dyn LowerExp),
    UpperExp(&'a dyn UpperExp),
    Pointer(&'a dyn Pointer),
    DebugLowerHex(&'a dyn Debug),
    DebugUpperHex(&'a dyn Debug),
}

pub enum Piece<'a> {
    Lit(&'static str),
    Arg(Argument<'a>, Spec),
}

pub struct Arguments<'a> {
    pub pieces: &'a [Piece<'a>],
}

impl<'a> Clone for Arguments<'a> {
    fn clone(&self) -> Arguments<'a> {
        Arguments { pieces: self.pieces }
    }
}
impl<'a> Copy for Arguments<'a> {}

impl<'a> Arguments<'a> {
    pub fn as_str(&self) -> Option<&'static str> {
        if self.pieces.len() == 0 {
            return Some("");
        }
        if self.pieces.len() == 1 {
            if let Piece::Lit(s) = &self.pieces[0] {
                return Some(*s);
            }
        }
        None
    }
}

impl<'a> Display for Arguments<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write(f.buf, *self)
    }
}
impl<'a> Debug for Arguments<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write(f.buf, *self)
    }
}

pub struct Formatter<'a> {
    pub(crate) buf: &'a mut dyn Write,
    pub(crate) spec: Spec,
    /// 0 = none, 1 = `{:x?}`, 2 = `{:X?}`
    pub(crate) debug_hex: u8,
}

pub trait Display {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}
pub trait Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}
pub trait LowerHex {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}
pub trait UpperHex {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}
pub trait Octal {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}
pub trait Binary {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}
pub trait LowerExp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}
pub trait UpperExp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}
pub trait Pointer {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result;
}

/// Format `args` into `output`.
pub fn write(output: &mut dyn Write, args: Arguments<'_>) -> Result {
    let pieces = args.pieces;
    let mut i = 0;
    while i < pieces.len() {
        match &pieces[i] {
            Piece::Lit(s) => output.write_str(*s)?,
            Piece::Arg(arg, spec) => {
                let mut f = Formatter { buf: &mut *output, spec: *spec, debug_hex: 0 };
                match arg {
                    Argument::Display(x) => x.fmt(&mut f)?,
                    Argument::Debug(x) => x.fmt(&mut f)?,
                    Argument::LowerHex(x) => x.fmt(&mut f)?,
                    Argument::UpperHex(x) => x.fmt(&mut f)?,
                    Argument::Octal(x) => x.fmt(&mut f)?,
                    Argument::Binary(x) => x.fmt(&mut f)?,
                    Argument::LowerExp(x) => x.fmt(&mut f)?,
                    Argument::UpperExp(x) => x.fmt(&mut f)?,
                    Argument::Pointer(x) => x.fmt(&mut f)?,
                    Argument::DebugLowerHex(x) => {
                        f.debug_hex = 1;
                        x.fmt(&mut f)?
                    }
                    Argument::DebugUpperHex(x) => {
                        f.debug_hex = 2;
                        x.fmt(&mut f)?
                    }
                }
            }
        }
        i += 1;
    }
    Ok(())
}

pub fn format(args: Arguments<'_>) -> String {
    let mut s = String::new();
    let _ = write(&mut s, args);
    s
}

fn utf8_count(s: &str) -> usize {
    let b = s.as_bytes();
    let mut n = 0;
    let mut i = 0;
    while i < b.len() {
        if (b[i] & 0xC0) != 0x80 {
            n += 1;
        }
        i += 1;
    }
    n
}

impl<'a> Formatter<'a> {
    pub fn new(buf: &'a mut dyn Write) -> Formatter<'a> {
        Formatter { buf, spec: DEFAULT_SPEC, debug_hex: 0 }
    }
    pub fn write_str(&mut self, s: &str) -> Result {
        self.buf.write_str(s)
    }
    pub fn write_char(&mut self, c: char) -> Result {
        self.buf.write_char(c)
    }
    pub fn write_fmt(&mut self, args: Arguments<'_>) -> Result {
        write(self.buf, args)
    }
    pub fn width(&self) -> Option<usize> {
        self.spec.width
    }
    pub fn precision(&self) -> Option<usize> {
        self.spec.precision
    }
    pub fn fill(&self) -> char {
        self.spec.fill
    }
    pub fn align(&self) -> Option<Align> {
        match self.spec.align {
            Align::Unknown => None,
            a => Some(a),
        }
    }
    pub fn alternate(&self) -> bool {
        self.spec.alternate
    }
    pub fn sign_plus(&self) -> bool {
        self.spec.sign_plus
    }
    pub fn sign_minus(&self) -> bool {
        self.spec.sign_minus
    }
    pub fn sign_aware_zero_pad(&self) -> bool {
        self.spec.zero_pad
    }

    fn write_fill(&mut self, n: usize) -> Result {
        let mut i = 0;
        while i < n {
            self.buf.write_char(self.spec.fill)?;
            i += 1;
        }
        Ok(())
    }

    /// Write `s` (of `len` chars) padded to the requested width.
    fn pad_with(&mut self, s: &str, len: usize, default: Align) -> Result {
        let width = match self.spec.width {
            Some(w) => w,
            None => return self.buf.write_str(s),
        };
        if len >= width {
            return self.buf.write_str(s);
        }
        let total = width - len;
        let align = match self.spec.align {
            Align::Unknown => default,
            a => a,
        };
        let (before, after) = match align {
            Align::Left => (0, total),
            Align::Right => (total, 0),
            Align::Center => (total / 2, total - total / 2),
            Align::Unknown => (0, total),
        };
        self.write_fill(before)?;
        self.buf.write_str(s)?;
        self.write_fill(after)
    }

    /// Pad a string: precision truncates, width pads (left-aligned by default).
    pub fn pad(&mut self, s: &str) -> Result {
        if self.spec.width.is_none() && self.spec.precision.is_none() {
            return self.buf.write_str(s);
        }
        let mut s = s;
        if let Some(max) = self.spec.precision {
            // truncate to `max` chars
            let b = s.as_bytes();
            let mut chars = 0;
            let mut i = 0;
            while i < b.len() {
                if (b[i] & 0xC0) != 0x80 {
                    if chars == max {
                        break;
                    }
                    chars += 1;
                }
                i += 1;
            }
            s = unsafe { crate::str::from_utf8_unchecked(crate::slice::from_raw_parts(b.as_ptr(), i)) };
        }
        let n = utf8_count(s);
        self.pad_with(s, n, Align::Left)
    }

    /// Pad an integer: sign, optional prefix (with `#`), digits.
    pub fn pad_integral(&mut self, is_nonnegative: bool, prefix: &str, digits: &str) -> Result {
        let sign = if !is_nonnegative {
            "-"
        } else if self.spec.sign_plus {
            "+"
        } else {
            ""
        };
        let prefix = if self.spec.alternate { prefix } else { "" };
        let len = sign.len() + prefix.len() + digits.len();
        let width = match self.spec.width {
            Some(w) => w,
            None => 0,
        };
        if len >= width {
            self.buf.write_str(sign)?;
            self.buf.write_str(prefix)?;
            return self.buf.write_str(digits);
        }
        if self.spec.zero_pad {
            self.buf.write_str(sign)?;
            self.buf.write_str(prefix)?;
            let mut i = len;
            while i < width {
                self.buf.write_str("0")?;
                i += 1;
            }
            return self.buf.write_str(digits);
        }
        let total = width - len;
        let (before, after) = match self.spec.align {
            Align::Left => (0, total),
            Align::Center => (total / 2, total - total / 2),
            _ => (total, 0),
        };
        self.write_fill(before)?;
        self.buf.write_str(sign)?;
        self.buf.write_str(prefix)?;
        self.buf.write_str(digits)?;
        self.write_fill(after)
    }

    pub fn debug_struct<'b>(&'b mut self, name: &str) -> DebugStruct<'b, 'a> {
        builders::debug_struct_new(self, name)
    }
    pub fn debug_tuple<'b>(&'b mut self, name: &str) -> DebugTuple<'b, 'a> {
        builders::debug_tuple_new(self, name)
    }
    pub fn debug_list<'b>(&'b mut self) -> DebugList<'b, 'a> {
        builders::debug_list_new(self)
    }
    pub fn debug_set<'b>(&'b mut self) -> DebugSet<'b, 'a> {
        builders::debug_set_new(self)
    }
    pub fn debug_map<'b>(&'b mut self) -> DebugMap<'b, 'a> {
        builders::debug_map_new(self)
    }
}

impl<'a> Write for Formatter<'a> {
    fn write_str(&mut self, s: &str) -> Result {
        self.buf.write_str(s)
    }
    fn write_char(&mut self, c: char) -> Result {
        self.buf.write_char(c)
    }
    fn write_fmt(&mut self, args: Arguments<'_>) -> Result {
        write(self.buf, args)
    }
}

// ---- references and basic types ----

macro_rules! fmt_refs {
    ($($tr:ident)*) => { $(
        impl<T: ?Sized + $tr> $tr for &T {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result { $tr::fmt(&**self, f) }
        }
        impl<T: ?Sized + $tr> $tr for &mut T {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result { $tr::fmt(&**self, f) }
        }
    )* };
}
fmt_refs! { Display Debug LowerHex UpperHex Octal Binary LowerExp UpperExp }

impl Display for str {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.pad(self)
    }
}

fn write_escaped_char(f: &mut Formatter<'_>, c: char, quote: char) -> Result {
    if c == '\n' {
        f.write_str("\\n")
    } else if c == '\r' {
        f.write_str("\\r")
    } else if c == '\t' {
        f.write_str("\\t")
    } else if c == '\\' {
        f.write_str("\\\\")
    } else if c == '\0' {
        f.write_str("\\0")
    } else if c == quote {
        f.write_char('\\')?;
        f.write_char(c)
    } else if crate::char::needs_unicode_escape(c) {
        f.write_str("\\u{")?;
        let mut buf = [0u8; 8];
        let mut n = c as u32;
        let mut i = 8;
        loop {
            i -= 1;
            let d = (n & 15) as u8;
            buf[i] = if d < 10 { b'0' + d } else { b'a' + d - 10 };
            n >>= 4;
            if n == 0 {
                break;
            }
        }
        f.write_str(unsafe { crate::str::from_utf8_unchecked(&buf[i..]) })?;
        f.write_str("}")
    } else {
        f.write_char(c)
    }
}

impl Debug for str {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_char('"')?;
        for c in self.chars() {
            write_escaped_char(f, c, '"')?;
        }
        f.write_char('"')
    }
}

impl Display for char {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut buf = [0u8; 4];
        let s: &str = self.encode_utf8(&mut buf);
        f.pad(s)
    }
}
impl Debug for char {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_char('\'')?;
        write_escaped_char(f, *self, '\'')?;
        f.write_char('\'')
    }
}

impl Display for bool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.pad(if *self { "true" } else { "false" })
    }
}
impl Debug for bool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.pad(if *self { "true" } else { "false" })
    }
}

impl Debug for () {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.pad("()")
    }
}

impl Display for ! {
    fn fmt(&self, _f: &mut Formatter<'_>) -> Result {
        *self
    }
}
impl Debug for ! {
    fn fmt(&self, _f: &mut Formatter<'_>) -> Result {
        *self
    }
}

impl<T: ?Sized> Debug for crate::marker::PhantomData<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("PhantomData<")?;
        f.write_str(crate::any::type_name::<T>())?;
        f.write_str(">")
    }
}

impl<T: Debug> Debug for [T] {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut l = f.debug_list();
        let mut i = 0;
        while i < self.len() {
            l.entry(&self[i]);
            i += 1;
        }
        l.finish()
    }
}
impl<T: Debug, const N: usize> Debug for [T; N] {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s: &[T] = self;
        Debug::fmt(s, f)
    }
}

impl<T: Debug> Debug for Option<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Some(x) => f.debug_tuple("Some").field(x).finish(),
            None => f.write_str("None"),
        }
    }
}
impl<T: Debug, E: Debug> Debug for crate::result::Result<T, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Ok(x) => f.debug_tuple("Ok").field(x).finish(),
            Err(e) => f.debug_tuple("Err").field(e).finish(),
        }
    }
}

impl<T: ?Sized> Pointer for *const T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let addr = *self as *const u8 as usize;
        let saved = f.spec.alternate;
        f.spec.alternate = true;
        let r = LowerHex::fmt(&addr, f);
        f.spec.alternate = saved;
        r
    }
}
impl<T: ?Sized> Pointer for *mut T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Pointer::fmt(&(*self as *const T), f)
    }
}
impl<T: ?Sized> Pointer for &T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Pointer::fmt(&(*self as *const T), f)
    }
}
impl<T: ?Sized> Pointer for &mut T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Pointer::fmt(&(&**self as *const T), f)
    }
}
impl<T: ?Sized> Debug for *const T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Pointer::fmt(self, f)
    }
}
impl<T: ?Sized> Debug for *mut T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Pointer::fmt(self, f)
    }
}

macro_rules! tuple_debug {
    ($( ($($name:ident $idx:tt)+) )*) => { $(
        impl<$($name: Debug),+> Debug for ($($name,)+) {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                let mut t = f.debug_tuple("");
                $( t.field(&self.$idx); )+
                t.finish()
            }
        }
    )* };
}
tuple_debug! {
    (A 0) (A 0 B 1) (A 0 B 1 C 2) (A 0 B 1 C 2 D 3) (A 0 B 1 C 2 D 3 E 4)
    (A 0 B 1 C 2 D 3 E 4 F 5) (A 0 B 1 C 2 D 3 E 4 F 5 G 6) (A 0 B 1 C 2 D 3 E 4 F 5 G 6 H 7)
}
