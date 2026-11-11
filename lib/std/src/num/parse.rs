use crate::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IntErrorKind {
    Empty,
    InvalidDigit,
    PosOverflow,
    NegOverflow,
    Zero,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParseIntError {
    kind: IntErrorKind,
}

impl ParseIntError {
    pub fn kind(&self) -> &IntErrorKind {
        &self.kind
    }
}

impl fmt::Display for ParseIntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.kind {
            IntErrorKind::Empty => "cannot parse integer from empty string",
            IntErrorKind::InvalidDigit => "invalid digit found in string",
            IntErrorKind::PosOverflow => "number too large to fit in target type",
            IntErrorKind::NegOverflow => "number too small to fit in target type",
            IntErrorKind::Zero => "number would be zero for non-zero type",
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParseFloatError {
    empty: bool,
}

impl fmt::Display for ParseFloatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.empty { "cannot parse float from empty string" } else { "invalid float literal" })
    }
}

pub(crate) fn float_error(empty: bool) -> ParseFloatError {
    ParseFloatError { empty }
}

/// Parse an integer in the given radix. `max` is the largest positive value
/// of the target type; signed targets accept `-` and values down to
/// -(max + 1). The result is the two's complement value as i128.
pub(crate) fn parse_int(src: &[u8], radix: u32, signed: bool, max: u128) -> Result<i128, ParseIntError> {
    if src.len() == 0 {
        return Err(ParseIntError { kind: IntErrorKind::Empty });
    }
    let mut i = 0;
    let mut neg = false;
    if src[0] == b'+' {
        i = 1;
    } else if src[0] == b'-' && signed {
        neg = true;
        i = 1;
    }
    if i == src.len() {
        return Err(ParseIntError { kind: IntErrorKind::InvalidDigit });
    }
    let limit: u128 = if neg { max + 1 } else { max };
    let mut acc: u128 = 0;
    while i < src.len() {
        let c = src[i];
        let d = if c >= b'0' && c <= b'9' {
            (c - b'0') as u32
        } else if c >= b'a' && c <= b'z' {
            (c - b'a') as u32 + 10
        } else if c >= b'A' && c <= b'Z' {
            (c - b'A') as u32 + 10
        } else {
            99
        };
        if d >= radix {
            return Err(ParseIntError { kind: IntErrorKind::InvalidDigit });
        }
        let over = if neg { IntErrorKind::NegOverflow } else { IntErrorKind::PosOverflow };
        if acc > limit / (radix as u128) {
            return Err(ParseIntError { kind: over });
        }
        acc = acc * (radix as u128);
        if acc > limit - (d as u128) {
            return Err(ParseIntError { kind: over });
        }
        acc = acc + d as u128;
        i += 1;
    }
    if neg { Ok((0u128 - acc) as i128) } else { Ok(acc as i128) }
}
