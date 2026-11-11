//! `char` methods. Non-ASCII classification covers the ranges that matter
//! for source text (Latin-1, Greek, Cyrillic, CJK, ...) rather than the full
//! Unicode tables.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CharTryFromError(pub(crate) ());

impl crate::fmt::Display for CharTryFromError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("converted integer out of range for `char`")
    }
}

pub fn from_u32(i: u32) -> Option<char> {
    if i > 0x10FFFF || (i >= 0xD800 && i <= 0xDFFF) { None } else { Some(unsafe { crate::intrinsics::transmute(i) }) }
}

pub unsafe fn from_u32_unchecked(i: u32) -> char {
    crate::intrinsics::transmute(i)
}

/// Chars that `{:?}` prints as `\u{..}`: controls, format/invisible
/// characters, surrogate-adjacent and other unassigned code points we know.
pub(crate) fn needs_unicode_escape(ch: char) -> bool {
    let c = ch as u32;
    c < 32 || c == 127 || (c >= 0x80 && c <= 0xA0) || c == 0xAD || c == 0x1680
        || (c >= 0x2000 && c <= 0x200A) || c == 0x202F || c == 0x205F || c == 0x3000
        || (c >= 0x200B && c <= 0x200F) || (c >= 0x2028 && c <= 0x202E) || (c >= 0x2060 && c <= 0x206F)
        || c == 0xFEFF || (c >= 0xFFF0 && c <= 0xFFFB) || (c >= 0xD7FC && c <= 0xD7FF)
        || (c >= 0xE000 && c <= 0xF8FF) || (c >= 0xFFFE && c <= 0xFFFF) || c >= 0xE0000
        || (c >= 0x300 && c <= 0x36F)
}

pub fn from_digit(num: u32, radix: u32) -> Option<char> {
    if num >= radix {
        return None;
    }
    let n = num as u8;
    Some(if n < 10 { (b'0' + n) as char } else { (b'a' + n - 10) as char })
}

fn is_alphabetic_wide(c: u32) -> bool {
    (c >= 0xC0 && c <= 0x24F && c != 0xD7 && c != 0xF7)
        || c == 0xAA || c == 0xB5 || c == 0xBA
        || (c >= 0x370 && c <= 0x3FF && c != 0x37E && c != 0x387)
        || (c >= 0x400 && c <= 0x52F && !(c >= 0x482 && c <= 0x489))
        || (c >= 0x531 && c <= 0x556) || (c >= 0x561 && c <= 0x587)
        || (c >= 0x5D0 && c <= 0x5EA)
        || (c >= 0x620 && c <= 0x64A)
        || (c >= 0x904 && c <= 0x939)
        || (c >= 0x1E00 && c <= 0x1FFF)
        || (c >= 0x3041 && c <= 0x3096) || (c >= 0x30A1 && c <= 0x30FA)
        || (c >= 0x3400 && c <= 0x4DBF) || (c >= 0x4E00 && c <= 0x9FFF)
        || (c >= 0xAC00 && c <= 0xD7A3)
        || (c >= 0x20000 && c <= 0x2A6DF)
}

impl char {
    pub const MAX: char = '\u{10FFFF}';
    pub const REPLACEMENT_CHARACTER: char = '\u{FFFD}';

    pub fn from_u32(i: u32) -> Option<char> {
        from_u32(i)
    }
    pub unsafe fn from_u32_unchecked(i: u32) -> char {
        from_u32_unchecked(i)
    }
    pub fn from_digit(num: u32, radix: u32) -> Option<char> {
        from_digit(num, radix)
    }
    pub const fn is_ascii(&self) -> bool {
        (*self as u32) < 128
    }
    pub const fn is_ascii_digit(&self) -> bool {
        *self >= '0' && *self <= '9'
    }
    pub const fn is_ascii_hexdigit(&self) -> bool {
        (*self >= '0' && *self <= '9') || (*self >= 'a' && *self <= 'f') || (*self >= 'A' && *self <= 'F')
    }
    pub const fn is_ascii_alphabetic(&self) -> bool {
        (*self >= 'a' && *self <= 'z') || (*self >= 'A' && *self <= 'Z')
    }
    pub const fn is_ascii_alphanumeric(&self) -> bool {
        self.is_ascii_alphabetic() || self.is_ascii_digit()
    }
    pub const fn is_ascii_uppercase(&self) -> bool {
        *self >= 'A' && *self <= 'Z'
    }
    pub const fn is_ascii_lowercase(&self) -> bool {
        *self >= 'a' && *self <= 'z'
    }
    pub const fn is_ascii_whitespace(&self) -> bool {
        *self == ' ' || *self == '\t' || *self == '\n' || *self == '\x0C' || *self == '\r'
    }
    pub const fn is_ascii_punctuation(&self) -> bool {
        let c = *self as u32;
        (c >= 33 && c <= 47) || (c >= 58 && c <= 64) || (c >= 91 && c <= 96) || (c >= 123 && c <= 126)
    }
    pub const fn is_ascii_graphic(&self) -> bool {
        let c = *self as u32;
        c >= 33 && c <= 126
    }
    pub const fn is_ascii_control(&self) -> bool {
        let c = *self as u32;
        c < 32 || c == 127
    }
    pub fn is_digit(self, radix: u32) -> bool {
        self.to_digit(radix).is_some()
    }
    pub fn to_digit(self, radix: u32) -> Option<u32> {
        let c = self as u32;
        let d = if c >= 48 && c <= 57 {
            c - 48
        } else if c >= 97 && c <= 122 {
            c - 97 + 10
        } else if c >= 65 && c <= 90 {
            c - 65 + 10
        } else {
            return None;
        };
        if d < radix { Some(d) } else { None }
    }
    pub fn is_alphabetic(self) -> bool {
        if self.is_ascii() { self.is_ascii_alphabetic() } else { is_alphabetic_wide(self as u32) }
    }
    pub fn is_numeric(self) -> bool {
        if self.is_ascii() {
            self.is_ascii_digit()
        } else {
            let c = self as u32;
            c == 0xB2 || c == 0xB3 || c == 0xB9 || (c >= 0xBC && c <= 0xBE) || (c >= 0x660 && c <= 0x669)
                || (c >= 0x966 && c <= 0x96F) || (c >= 0x2160 && c <= 0x2188) || (c >= 0xFF10 && c <= 0xFF19)
        }
    }
    pub fn is_alphanumeric(self) -> bool {
        self.is_alphabetic() || self.is_numeric()
    }
    pub fn is_whitespace(self) -> bool {
        let c = self as u32;
        c == 32 || (c >= 9 && c <= 13) || c == 0x85 || c == 0xA0 || c == 0x1680 || (c >= 0x2000 && c <= 0x200A)
            || c == 0x2028 || c == 0x2029 || c == 0x202F || c == 0x205F || c == 0x3000
    }
    pub fn is_control(self) -> bool {
        let c = self as u32;
        c < 32 || (c >= 127 && c < 160)
    }
    pub fn is_uppercase(self) -> bool {
        let c = self as u32;
        if c < 128 {
            self.is_ascii_uppercase()
        } else {
            (c >= 0xC0 && c <= 0xDE && c != 0xD7) || (c >= 0x391 && c <= 0x3A9) || (c >= 0x410 && c <= 0x42F)
                || (c >= 0x100 && c <= 0x17F && c % 2 == 0)
        }
    }
    pub fn is_lowercase(self) -> bool {
        let c = self as u32;
        if c < 128 {
            self.is_ascii_lowercase()
        } else {
            (c >= 0xDF && c <= 0xFF && c != 0xF7) || (c >= 0x3B1 && c <= 0x3C9) || (c >= 0x430 && c <= 0x44F)
                || (c >= 0x100 && c <= 0x17F && c % 2 == 1)
        }
    }
    pub const fn to_ascii_uppercase(&self) -> char {
        if self.is_ascii_lowercase() { ((*self as u8) - 32) as char } else { *self }
    }
    pub const fn to_ascii_lowercase(&self) -> char {
        if self.is_ascii_uppercase() { ((*self as u8) + 32) as char } else { *self }
    }
    pub fn make_ascii_uppercase(&mut self) {
        *self = self.to_ascii_uppercase()
    }
    pub fn make_ascii_lowercase(&mut self) {
        *self = self.to_ascii_lowercase()
    }
    pub fn eq_ignore_ascii_case(&self, other: &char) -> bool {
        self.to_ascii_lowercase() == other.to_ascii_lowercase()
    }
    /// Simple (one-to-one) lowercase mapping.
    pub(crate) fn to_lower_simple(self) -> char {
        let c = self as u32;
        if c < 128 {
            return self.to_ascii_lowercase();
        }
        let r = if (c >= 0xC0 && c <= 0xDE && c != 0xD7) || (c >= 0x391 && c <= 0x3A9 && c != 0x3A2) || (c >= 0x410 && c <= 0x42F) {
            c + 32
        } else if c >= 0x400 && c <= 0x40F {
            c + 80
        } else if c >= 0x100 && c <= 0x17F && c % 2 == 0 && c != 0x130 && c != 0x138 {
            c + 1
        } else {
            c
        };
        unsafe { from_u32_unchecked(r) }
    }
    pub(crate) fn to_upper_simple(self) -> char {
        let c = self as u32;
        if c < 128 {
            return self.to_ascii_uppercase();
        }
        let r = if (c >= 0xE0 && c <= 0xFE && c != 0xF7) || (c >= 0x3B1 && c <= 0x3C9 && c != 0x3C2) || (c >= 0x430 && c <= 0x44F) {
            c - 32
        } else if c >= 0x450 && c <= 0x45F {
            c - 80
        } else if c >= 0x100 && c <= 0x17F && c % 2 == 1 && c != 0x131 && c != 0x149 {
            c - 1
        } else {
            c
        };
        unsafe { from_u32_unchecked(r) }
    }
    pub fn to_lowercase(self) -> ToLowercase {
        ToLowercase { buf: CharBuf::one(self.to_lower_simple()) }
    }
    pub fn to_uppercase(self) -> ToUppercase {
        if self == 'ß' {
            let mut b = CharBuf::one('S');
            b.push('S');
            return ToUppercase { buf: b };
        }
        ToUppercase { buf: CharBuf::one(self.to_upper_simple()) }
    }
    /// `\u{NNNN}`
    pub fn escape_unicode(self) -> EscapeUnicode {
        let mut b = CharBuf::empty();
        b.push_unicode(self);
        EscapeUnicode { buf: b }
    }
    /// Escapes like `{:?}` does (without the quotes).
    pub fn escape_debug(self) -> EscapeDebug {
        let mut b = CharBuf::empty();
        let c = self as u32;
        if self == '\n' {
            b.push('\\');
            b.push('n');
        } else if self == '\r' {
            b.push('\\');
            b.push('r');
        } else if self == '\t' {
            b.push('\\');
            b.push('t');
        } else if self == '\0' {
            b.push('\\');
            b.push('0');
        } else if self == '\\' || self == '\'' || self == '"' {
            b.push('\\');
            b.push(self);
        } else if needs_unicode_escape(self) {
            b.push_unicode(self);
        } else {
            b.push(self);
        }
        EscapeDebug { buf: b }
    }
    /// Escapes everything outside printable ASCII.
    pub fn escape_default(self) -> EscapeDefault {
        let mut b = CharBuf::empty();
        let c = self as u32;
        if self == '\n' {
            b.push('\\');
            b.push('n');
        } else if self == '\r' {
            b.push('\\');
            b.push('r');
        } else if self == '\t' {
            b.push('\\');
            b.push('t');
        } else if self == '\\' || self == '\'' || self == '"' {
            b.push('\\');
            b.push(self);
        } else if c >= 32 && c < 127 {
            b.push(self);
        } else {
            b.push_unicode(self);
        }
        EscapeDefault { buf: b }
    }
    pub const fn len_utf8(self) -> usize {
        let c = self as u32;
        if c < 0x80 {
            1
        } else if c < 0x800 {
            2
        } else if c < 0x10000 {
            3
        } else {
            4
        }
    }
    pub const fn len_utf16(self) -> usize {
        if (self as u32) < 0x10000 { 1 } else { 2 }
    }
    pub fn encode_utf8(self, dst: &mut [u8]) -> &mut str {
        let c = self as u32;
        let n = self.len_utf8();
        if n == 1 {
            dst[0] = c as u8;
        } else if n == 2 {
            dst[0] = (0xC0 | (c >> 6)) as u8;
            dst[1] = (0x80 | (c & 0x3F)) as u8;
        } else if n == 3 {
            dst[0] = (0xE0 | (c >> 12)) as u8;
            dst[1] = (0x80 | ((c >> 6) & 0x3F)) as u8;
            dst[2] = (0x80 | (c & 0x3F)) as u8;
        } else {
            dst[0] = (0xF0 | (c >> 18)) as u8;
            dst[1] = (0x80 | ((c >> 12) & 0x3F)) as u8;
            dst[2] = (0x80 | ((c >> 6) & 0x3F)) as u8;
            dst[3] = (0x80 | (c & 0x3F)) as u8;
        }
        unsafe { crate::str::from_utf8_unchecked_mut(&mut dst[..n]) }
    }
}

/// A few chars, iterated front to back.
#[derive(Clone)]
struct CharBuf {
    chars: [char; 10],
    len: usize,
    pos: usize,
}

impl CharBuf {
    fn empty() -> CharBuf {
        CharBuf { chars: ['\0'; 10], len: 0, pos: 0 }
    }
    fn one(c: char) -> CharBuf {
        let mut b = CharBuf::empty();
        b.push(c);
        b
    }
    fn push(&mut self, c: char) {
        self.chars[self.len] = c;
        self.len += 1;
    }
    fn push_unicode(&mut self, c: char) {
        self.push('\\');
        self.push('u');
        self.push('{');
        let n = c as u32;
        let mut shift: u32 = 20;
        let mut started = false;
        loop {
            let d = (n >> shift) & 15;
            if d != 0 || started || shift == 0 {
                started = true;
                self.push(from_digit(d, 16).unwrap());
            }
            if shift == 0 {
                break;
            }
            shift -= 4;
        }
        self.push('}');
    }
    fn next(&mut self) -> Option<char> {
        if self.pos < self.len {
            let c = self.chars[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        let mut i = self.pos;
        while i < self.len {
            f.write_char(self.chars[i])?;
            i += 1;
        }
        Ok(())
    }
}

macro_rules! char_iter {
    ($($name:ident)*) => { $(
        #[derive(Clone)]
        pub struct $name {
            buf: CharBuf,
        }
        impl Iterator for $name {
            type Item = char;
            fn next(&mut self) -> Option<char> {
                self.buf.next()
            }
        }
        impl crate::fmt::Display for $name {
            fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
                self.buf.fmt(f)
            }
        }
    )* };
}
char_iter! { ToLowercase ToUppercase EscapeUnicode EscapeDebug EscapeDefault }
