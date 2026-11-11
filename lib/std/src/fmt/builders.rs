//! `Formatter::debug_struct` and friends, including `{:#?}` pretty printing.

use crate::fmt::{Debug, Formatter, Result, Write};

/// Indents everything written through it by four spaces per line.
struct PadAdapter<'a> {
    buf: &'a mut dyn Write,
    on_newline: bool,
}

impl<'a> Write for PadAdapter<'a> {
    fn write_str(&mut self, s: &str) -> Result {
        let b = s.as_bytes();
        let mut start = 0;
        let mut i = 0;
        while i < b.len() {
            if self.on_newline {
                self.buf.write_str("    ")?;
                self.on_newline = false;
            }
            if b[i] == b'\n' {
                self.buf.write_str(unsafe { crate::str::from_utf8_unchecked(&b[start..i + 1]) })?;
                start = i + 1;
                self.on_newline = true;
            }
            i += 1;
        }
        if start < b.len() {
            if self.on_newline {
                self.buf.write_str("    ")?;
                self.on_newline = false;
            }
            self.buf.write_str(unsafe { crate::str::from_utf8_unchecked(&b[start..]) })?;
        }
        Ok(())
    }
}

/// Format `value` (and `suffix`) indented, for alternate mode.
fn write_padded(fmt: &mut Formatter<'_>, prefix: &str, value: &dyn Debug, suffix: &str) -> Result {
    let spec = fmt.spec;
    let debug_hex = fmt.debug_hex;
    let mut pad = PadAdapter { buf: &mut *fmt.buf, on_newline: true };
    pad.write_str(prefix)?;
    {
        let mut inner = Formatter { buf: &mut pad, spec, debug_hex };
        value.fmt(&mut inner)?;
    }
    pad.write_str(suffix)
}

pub struct DebugStruct<'a, 'b> {
    fmt: &'a mut Formatter<'b>,
    result: Result,
    has_fields: bool,
}

pub(crate) fn debug_struct_new<'a, 'b>(fmt: &'a mut Formatter<'b>, name: &str) -> DebugStruct<'a, 'b> {
    let result = fmt.write_str(name);
    DebugStruct { fmt, result, has_fields: false }
}

impl<'a, 'b> DebugStruct<'a, 'b> {
    pub fn field(&mut self, name: &str, value: &dyn Debug) -> &mut DebugStruct<'a, 'b> {
        if self.result.is_ok() {
            self.result = self.field_inner(name, value);
        }
        self.has_fields = true;
        self
    }
    fn field_inner(&mut self, name: &str, value: &dyn Debug) -> Result {
        if self.fmt.alternate() {
            if !self.has_fields {
                self.fmt.write_str(" {\n")?;
            }
            let spec = self.fmt.spec;
            let debug_hex = self.fmt.debug_hex;
            let mut pad = PadAdapter { buf: &mut *self.fmt.buf, on_newline: true };
            pad.write_str(name)?;
            pad.write_str(": ")?;
            {
                let mut inner = Formatter { buf: &mut pad, spec, debug_hex };
                value.fmt(&mut inner)?;
            }
            pad.write_str(",\n")
        } else {
            self.fmt.write_str(if self.has_fields { ", " } else { " { " })?;
            self.fmt.write_str(name)?;
            self.fmt.write_str(": ")?;
            value.fmt(self.fmt)
        }
    }
    pub fn finish_non_exhaustive(&mut self) -> Result {
        if self.result.is_ok() {
            self.result = if self.has_fields {
                if self.fmt.alternate() {
                    let mut pad = PadAdapter { buf: &mut *self.fmt.buf, on_newline: true };
                    match pad.write_str("..\n") {
                        Ok(()) => self.fmt.write_str("}"),
                        Err(e) => Err(e),
                    }
                } else {
                    self.fmt.write_str(", .. }")
                }
            } else {
                self.fmt.write_str(" { .. }")
            };
        }
        self.result
    }
    pub fn finish(&mut self) -> Result {
        if self.has_fields && self.result.is_ok() {
            self.result = if self.fmt.alternate() { self.fmt.write_str("}") } else { self.fmt.write_str(" }") };
        }
        self.result
    }
}

pub struct DebugTuple<'a, 'b> {
    fmt: &'a mut Formatter<'b>,
    result: Result,
    fields: usize,
    empty_name: bool,
}

pub(crate) fn debug_tuple_new<'a, 'b>(fmt: &'a mut Formatter<'b>, name: &str) -> DebugTuple<'a, 'b> {
    let result = fmt.write_str(name);
    DebugTuple { fmt, result, fields: 0, empty_name: name.len() == 0 }
}

impl<'a, 'b> DebugTuple<'a, 'b> {
    pub fn field(&mut self, value: &dyn Debug) -> &mut DebugTuple<'a, 'b> {
        if self.result.is_ok() {
            self.result = self.field_inner(value);
        }
        self.fields += 1;
        self
    }
    fn field_inner(&mut self, value: &dyn Debug) -> Result {
        if self.fmt.alternate() {
            if self.fields == 0 {
                self.fmt.write_str("(\n")?;
            }
            write_padded(self.fmt, "", value, ",\n")
        } else {
            self.fmt.write_str(if self.fields == 0 { "(" } else { ", " })?;
            value.fmt(self.fmt)
        }
    }
    pub fn finish(&mut self) -> Result {
        if self.fields > 0 && self.result.is_ok() {
            if self.fields == 1 && self.empty_name && !self.fmt.alternate() {
                self.result = self.fmt.write_str(",");
            }
            if self.result.is_ok() {
                self.result = self.fmt.write_str(")");
            }
        }
        self.result
    }
}

struct DebugInner<'a, 'b> {
    fmt: &'a mut Formatter<'b>,
    result: Result,
    has_fields: bool,
}

impl<'a, 'b> DebugInner<'a, 'b> {
    fn entry(&mut self, value: &dyn Debug) {
        if self.result.is_ok() {
            self.result = self.entry_inner(value);
        }
        self.has_fields = true;
    }
    fn entry_inner(&mut self, value: &dyn Debug) -> Result {
        if self.fmt.alternate() {
            if !self.has_fields {
                self.fmt.write_str("\n")?;
            }
            write_padded(self.fmt, "", value, ",\n")
        } else {
            if self.has_fields {
                self.fmt.write_str(", ")?;
            }
            value.fmt(self.fmt)
        }
    }
}

pub struct DebugList<'a, 'b> {
    inner: DebugInner<'a, 'b>,
}

pub(crate) fn debug_list_new<'a, 'b>(fmt: &'a mut Formatter<'b>) -> DebugList<'a, 'b> {
    let result = fmt.write_str("[");
    DebugList { inner: DebugInner { fmt, result, has_fields: false } }
}

impl<'a, 'b> DebugList<'a, 'b> {
    pub fn entry(&mut self, value: &dyn Debug) -> &mut DebugList<'a, 'b> {
        self.inner.entry(value);
        self
    }
    pub fn entries<D: Debug, I: IntoIterator<Item = D>>(&mut self, entries: I) -> &mut DebugList<'a, 'b> {
        for e in entries {
            self.inner.entry(&e);
        }
        self
    }
    pub fn finish(&mut self) -> Result {
        if self.inner.result.is_ok() {
            self.inner.result = self.inner.fmt.write_str("]");
        }
        self.inner.result
    }
}

pub struct DebugSet<'a, 'b> {
    inner: DebugInner<'a, 'b>,
}

pub(crate) fn debug_set_new<'a, 'b>(fmt: &'a mut Formatter<'b>) -> DebugSet<'a, 'b> {
    let result = fmt.write_str("{");
    DebugSet { inner: DebugInner { fmt, result, has_fields: false } }
}

impl<'a, 'b> DebugSet<'a, 'b> {
    pub fn entry(&mut self, value: &dyn Debug) -> &mut DebugSet<'a, 'b> {
        self.inner.entry(value);
        self
    }
    pub fn entries<D: Debug, I: IntoIterator<Item = D>>(&mut self, entries: I) -> &mut DebugSet<'a, 'b> {
        for e in entries {
            self.inner.entry(&e);
        }
        self
    }
    pub fn finish(&mut self) -> Result {
        if self.inner.result.is_ok() {
            self.inner.result = self.inner.fmt.write_str("}");
        }
        self.inner.result
    }
}

pub struct DebugMap<'a, 'b> {
    fmt: &'a mut Formatter<'b>,
    result: Result,
    has_fields: bool,
}

pub(crate) fn debug_map_new<'a, 'b>(fmt: &'a mut Formatter<'b>) -> DebugMap<'a, 'b> {
    let result = fmt.write_str("{");
    DebugMap { fmt, result, has_fields: false }
}

impl<'a, 'b> DebugMap<'a, 'b> {
    pub fn entry(&mut self, key: &dyn Debug, value: &dyn Debug) -> &mut DebugMap<'a, 'b> {
        if self.result.is_ok() {
            self.result = self.entry_inner(key, value);
        }
        self.has_fields = true;
        self
    }
    fn entry_inner(&mut self, key: &dyn Debug, value: &dyn Debug) -> Result {
        if self.fmt.alternate() {
            if !self.has_fields {
                self.fmt.write_str("\n")?;
            }
            let spec = self.fmt.spec;
            let debug_hex = self.fmt.debug_hex;
            let mut pad = PadAdapter { buf: &mut *self.fmt.buf, on_newline: true };
            {
                let mut inner = Formatter { buf: &mut pad, spec, debug_hex };
                key.fmt(&mut inner)?;
            }
            pad.write_str(": ")?;
            {
                let mut inner = Formatter { buf: &mut pad, spec, debug_hex };
                value.fmt(&mut inner)?;
            }
            pad.write_str(",\n")
        } else {
            if self.has_fields {
                self.fmt.write_str(", ")?;
            }
            key.fmt(self.fmt)?;
            self.fmt.write_str(": ")?;
            value.fmt(self.fmt)
        }
    }
    pub fn entries<K: Debug, V: Debug, I: IntoIterator<Item = (K, V)>>(&mut self, entries: I) -> &mut DebugMap<'a, 'b> {
        for (k, v) in entries {
            self.entry(&k, &v);
        }
        self
    }
    pub fn finish(&mut self) -> Result {
        if self.result.is_ok() {
            self.result = self.fmt.write_str("}");
        }
        self.result
    }
}

// DebugMap::key / value (added with std::collections).
impl<'a, 'b> DebugMap<'a, 'b> {
    pub fn key(&mut self, key: &dyn Debug) -> &mut DebugMap<'a, 'b> {
        if self.result.is_ok() {
            self.result = self.key_inner(key);
        }
        self
    }
    fn key_inner(&mut self, key: &dyn Debug) -> Result {
        if self.fmt.alternate() {
            if !self.has_fields {
                self.fmt.write_str("\n")?;
            }
            let spec = self.fmt.spec;
            let debug_hex = self.fmt.debug_hex;
            let mut pad = PadAdapter { buf: &mut *self.fmt.buf, on_newline: true };
            {
                let mut inner = Formatter { buf: &mut pad, spec, debug_hex };
                key.fmt(&mut inner)?;
            }
            pad.write_str(": ")
        } else {
            if self.has_fields {
                self.fmt.write_str(", ")?;
            }
            key.fmt(self.fmt)?;
            self.fmt.write_str(": ")
        }
    }
    pub fn value(&mut self, value: &dyn Debug) -> &mut DebugMap<'a, 'b> {
        if self.result.is_ok() {
            self.result = self.value_inner(value);
        }
        self.has_fields = true;
        self
    }
    fn value_inner(&mut self, value: &dyn Debug) -> Result {
        if self.fmt.alternate() {
            let spec = self.fmt.spec;
            let debug_hex = self.fmt.debug_hex;
            let mut pad = PadAdapter { buf: &mut *self.fmt.buf, on_newline: false };
            {
                let mut inner = Formatter { buf: &mut pad, spec, debug_hex };
                value.fmt(&mut inner)?;
            }
            pad.write_str(",\n")
        } else {
            value.fmt(self.fmt)
        }
    }
}
