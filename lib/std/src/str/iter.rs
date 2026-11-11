use crate::iter::{DoubleEndedIterator, Iterator};
use crate::str::{decode_at, decode_before, Pattern};

pub struct Chars<'a> {
    s: &'a str,
    front: usize,
    back: usize,
}
pub(crate) fn chars_new(s: &str) -> Chars<'_> {
    Chars { s, front: 0, back: s.len() }
}
impl<'a> Chars<'a> {
    pub fn as_str(&self) -> &'a str {
        unsafe { self.s.get_unchecked(self.front, self.back) }
    }
}
impl<'a> Clone for Chars<'a> {
    fn clone(&self) -> Chars<'a> {
        Chars { s: self.s, front: self.front, back: self.back }
    }
}
impl<'a> Iterator for Chars<'a> {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if self.front >= self.back {
            return None;
        }
        let (c, n) = decode_at(self.s.as_bytes(), self.front);
        self.front += n;
        Some(c)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        ((n + 3) / 4, Some(n))
    }
}
impl<'a> DoubleEndedIterator for Chars<'a> {
    fn next_back(&mut self) -> Option<char> {
        if self.front >= self.back {
            return None;
        }
        let (c, start) = decode_before(self.s.as_bytes(), self.back);
        self.back = start;
        Some(c)
    }
}

pub struct CharIndices<'a> {
    chars: Chars<'a>,
}
pub(crate) fn char_indices_new(s: &str) -> CharIndices<'_> {
    CharIndices { chars: chars_new(s) }
}
impl<'a> CharIndices<'a> {
    pub fn as_str(&self) -> &'a str {
        self.chars.as_str()
    }
    pub fn offset(&self) -> usize {
        self.chars.front
    }
}
impl<'a> Clone for CharIndices<'a> {
    fn clone(&self) -> CharIndices<'a> {
        CharIndices { chars: self.chars.clone() }
    }
}
impl<'a> Iterator for CharIndices<'a> {
    type Item = (usize, char);
    fn next(&mut self) -> Option<(usize, char)> {
        let i = self.chars.front;
        match self.chars.next() {
            Some(c) => Some((i, c)),
            None => None,
        }
    }
}
impl<'a> DoubleEndedIterator for CharIndices<'a> {
    fn next_back(&mut self) -> Option<(usize, char)> {
        match self.chars.next_back() {
            Some(c) => Some((self.chars.back, c)),
            None => None,
        }
    }
}

pub struct Bytes<'a> {
    b: &'a [u8],
    front: usize,
    back: usize,
}
pub(crate) fn bytes_new(s: &str) -> Bytes<'_> {
    Bytes { b: s.as_bytes(), front: 0, back: s.len() }
}
impl<'a> Clone for Bytes<'a> {
    fn clone(&self) -> Bytes<'a> {
        Bytes { b: self.b, front: self.front, back: self.back }
    }
}
impl<'a> Iterator for Bytes<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        if self.front >= self.back {
            return None;
        }
        let x = self.b[self.front];
        self.front += 1;
        Some(x)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.back - self.front, Some(self.back - self.front))
    }
}
impl<'a> DoubleEndedIterator for Bytes<'a> {
    fn next_back(&mut self) -> Option<u8> {
        if self.front >= self.back {
            return None;
        }
        self.back -= 1;
        Some(self.b[self.back])
    }
}
impl<'a> crate::iter::ExactSizeIterator for Bytes<'a> {}

/// Shared state of the forward splitters.
pub struct Split<'a, P> {
    s: &'a str,
    pos: usize,
    /// where the next search starts (past `pos` after an empty match)
    search: usize,
    end: usize,
    finished: bool,
    pat: P,
}
pub(crate) fn split_new<'a, P>(s: &'a str, pat: P) -> Split<'a, P> {
    Split { s, pos: 0, search: 0, end: s.len(), finished: false, pat }
}
impl<'a, P: Pattern> Split<'a, P> {
    fn rest(&mut self) -> Option<&'a str> {
        if self.finished {
            return None;
        }
        self.finished = true;
        Some(unsafe { self.s.get_unchecked(self.pos, self.end) })
    }
    pub fn remainder(&self) -> Option<&'a str> {
        if self.finished { None } else { Some(unsafe { self.s.get_unchecked(self.pos, self.end) }) }
    }
    /// Empty-width matches are skipped at the current position.
    fn next_match(&mut self) -> Option<(usize, usize)> {
        if self.search > self.end {
            return None;
        }
        let hay = unsafe { self.s.get_unchecked(0, self.end) };
        match self.pat.find_in(hay, self.search) {
            Some((a, b)) => {
                if b > a {
                    self.search = b;
                } else if b < self.end {
                    // empty match: the next search starts one char later
                    self.search = b + decode_at(self.s.as_bytes(), b).1;
                } else {
                    self.search = self.end + 1;
                }
                Some((a, b))
            }
            None => None,
        }
    }
}
impl<'a, P: Pattern> Iterator for Split<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.finished {
            return None;
        }
        match self.next_match() {
            Some((a, b)) => {
                let r = unsafe { self.s.get_unchecked(self.pos, a) };
                self.pos = b;
                Some(r)
            }
            None => self.rest(),
        }
    }
}
impl<'a, P: Pattern> DoubleEndedIterator for Split<'a, P> {
    fn next_back(&mut self) -> Option<&'a str> {
        if self.finished {
            return None;
        }
        let hay = unsafe { self.s.get_unchecked(self.pos, self.end) };
        match self.pat.rfind_in(hay, hay.len()) {
            Some((a, b)) => {
                let r = unsafe { self.s.get_unchecked(self.pos + b, self.end) };
                self.end = self.pos + a;
                Some(r)
            }
            None => self.rest(),
        }
    }
}

pub struct RSplit<'a, P> {
    inner: Split<'a, P>,
}
pub(crate) fn rsplit_new<'a, P>(s: &'a str, pat: P) -> RSplit<'a, P> {
    RSplit { inner: split_new(s, pat) }
}
impl<'a, P: Pattern> Iterator for RSplit<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        self.inner.next_back()
    }
}

pub struct SplitN<'a, P> {
    inner: Split<'a, P>,
    n: usize,
}
pub(crate) fn splitn_new<'a, P>(s: &'a str, n: usize, pat: P) -> SplitN<'a, P> {
    SplitN { inner: split_new(s, pat), n }
}
impl<'a, P: Pattern> Iterator for SplitN<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        if self.n == 0 { self.inner.rest() } else { self.inner.next() }
    }
}

pub struct RSplitN<'a, P> {
    inner: Split<'a, P>,
    n: usize,
}
pub(crate) fn rsplitn_new<'a, P>(s: &'a str, n: usize, pat: P) -> RSplitN<'a, P> {
    RSplitN { inner: split_new(s, pat), n }
}
impl<'a, P: Pattern> Iterator for RSplitN<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        if self.n == 0 { self.inner.rest() } else { self.inner.next_back() }
    }
}

pub struct SplitTerminator<'a, P> {
    inner: Split<'a, P>,
}
pub(crate) fn split_terminator_new<'a, P>(s: &'a str, pat: P) -> SplitTerminator<'a, P> {
    SplitTerminator { inner: split_new(s, pat) }
}
impl<'a, P: Pattern> Iterator for SplitTerminator<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.inner.finished {
            return None;
        }
        match self.inner.next_match() {
            Some((a, b)) => {
                let r = unsafe { self.inner.s.get_unchecked(self.inner.pos, a) };
                self.inner.pos = b;
                Some(r)
            }
            None => {
                self.inner.finished = true;
                if self.inner.pos < self.inner.end {
                    Some(unsafe { self.inner.s.get_unchecked(self.inner.pos, self.inner.end) })
                } else {
                    None
                }
            }
        }
    }
}

pub struct SplitInclusive<'a, P> {
    inner: Split<'a, P>,
}
pub(crate) fn split_inclusive_new<'a, P>(s: &'a str, pat: P) -> SplitInclusive<'a, P> {
    SplitInclusive { inner: split_new(s, pat) }
}
impl<'a, P: Pattern> Iterator for SplitInclusive<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.inner.finished {
            return None;
        }
        match self.inner.next_match() {
            Some((_, b)) => {
                let r = unsafe { self.inner.s.get_unchecked(self.inner.pos, b) };
                self.inner.pos = b;
                Some(r)
            }
            None => {
                self.inner.finished = true;
                if self.inner.pos < self.inner.end {
                    Some(unsafe { self.inner.s.get_unchecked(self.inner.pos, self.inner.end) })
                } else {
                    None
                }
            }
        }
    }
}

pub struct Lines<'a> {
    inner: SplitTerminator<'a, char>,
}
pub(crate) fn lines_new(s: &str) -> Lines<'_> {
    Lines { inner: split_terminator_new(s, '\n') }
}
impl<'a> Iterator for Lines<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        match self.inner.next() {
            Some(line) => {
                let b = line.as_bytes();
                if b.len() > 0 && b[b.len() - 1] == b'\r' {
                    Some(unsafe { line.get_unchecked(0, b.len() - 1) })
                } else {
                    Some(line)
                }
            }
            None => None,
        }
    }
}

pub struct SplitWhitespace<'a> {
    s: &'a str,
    pos: usize,
}
pub(crate) fn split_whitespace_new(s: &str) -> SplitWhitespace<'_> {
    SplitWhitespace { s, pos: 0 }
}
impl<'a> Iterator for SplitWhitespace<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        let b = self.s.as_bytes();
        while self.pos < b.len() {
            let (c, n) = decode_at(b, self.pos);
            if !c.is_whitespace() {
                break;
            }
            self.pos += n;
        }
        if self.pos >= b.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < b.len() {
            let (c, n) = decode_at(b, self.pos);
            if c.is_whitespace() {
                break;
            }
            self.pos += n;
        }
        Some(unsafe { self.s.get_unchecked(start, self.pos) })
    }
}
impl<'a> DoubleEndedIterator for SplitWhitespace<'a> {
    fn next_back(&mut self) -> Option<&'a str> {
        let rest = unsafe { self.s.get_unchecked(self.pos, self.s.len()) }.trim_end();
        if rest.len() == 0 {
            return None;
        }
        let b = rest.as_bytes();
        let mut start = b.len();
        while start > 0 {
            let (c, s0) = decode_before(b, start);
            if c.is_whitespace() {
                break;
            }
            start = s0;
        }
        let word = unsafe { rest.get_unchecked(start, b.len()) };
        self.s = unsafe { self.s.get_unchecked(0, self.pos + start) };
        Some(word)
    }
}

pub struct SplitAsciiWhitespace<'a> {
    inner: SplitWhitespace<'a>,
}
pub(crate) fn split_ascii_whitespace_new(s: &str) -> SplitAsciiWhitespace<'_> {
    SplitAsciiWhitespace { inner: split_whitespace_new(s) }
}
impl<'a> Iterator for SplitAsciiWhitespace<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        self.inner.next()
    }
}

pub struct MatchIndices<'a, P> {
    s: &'a str,
    pos: usize,
    pat: P,
}
pub(crate) fn match_indices_new<'a, P>(s: &'a str, pat: P) -> MatchIndices<'a, P> {
    MatchIndices { s, pos: 0, pat }
}
impl<'a, P: Pattern> Iterator for MatchIndices<'a, P> {
    type Item = (usize, &'a str);
    fn next(&mut self) -> Option<(usize, &'a str)> {
        if self.pos > self.s.len() {
            return None;
        }
        match self.pat.find_in(self.s, self.pos) {
            Some((a, b)) => {
                self.pos = if b > a { b } else { b + 1 };
                Some((a, unsafe { self.s.get_unchecked(a, b) }))
            }
            None => {
                self.pos = self.s.len() + 1;
                None
            }
        }
    }
}

pub struct Matches<'a, P> {
    inner: MatchIndices<'a, P>,
}
pub(crate) fn matches_new<'a, P>(s: &'a str, pat: P) -> Matches<'a, P> {
    Matches { inner: match_indices_new(s, pat) }
}
impl<'a, P: Pattern> Iterator for Matches<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        match self.inner.next() {
            Some((_, m)) => Some(m),
            None => None,
        }
    }
}
