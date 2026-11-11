//! String search patterns: `char`, `&str`, `&String`, `[char; N]`,
//! `&[char]` and `FnMut(char) -> bool`.

use crate::str::{decode_at, decode_before};

pub trait Pattern {
    /// First match starting at byte `from` or later: (start, end).
    fn find_in(&mut self, hay: &str, from: usize) -> Option<(usize, usize)>;
    /// Last match ending at byte `to` or earlier.
    fn rfind_in(&mut self, hay: &str, to: usize) -> Option<(usize, usize)>;
    /// Length of the match at the start of `hay`, if any.
    fn prefix_len(&mut self, hay: &str) -> Option<usize>;
    /// Length of the match at the end of `hay`, if any.
    fn suffix_len(&mut self, hay: &str) -> Option<usize>;
}

fn find_char_pred<F: FnMut(char) -> bool>(hay: &str, from: usize, f: &mut F) -> Option<(usize, usize)> {
    let b = hay.as_bytes();
    let mut i = from;
    while i < b.len() {
        let (c, n) = decode_at(b, i);
        if f(c) {
            return Some((i, i + n));
        }
        i += n;
    }
    None
}

fn rfind_char_pred<F: FnMut(char) -> bool>(hay: &str, to: usize, f: &mut F) -> Option<(usize, usize)> {
    let b = hay.as_bytes();
    let mut end = to;
    while end > 0 {
        let (c, start) = decode_before(b, end);
        if f(c) {
            return Some((start, end));
        }
        end = start;
    }
    None
}

fn prefix_char_pred<F: FnMut(char) -> bool>(hay: &str, f: &mut F) -> Option<usize> {
    if hay.len() == 0 {
        return None;
    }
    let (c, n) = decode_at(hay.as_bytes(), 0);
    if f(c) { Some(n) } else { None }
}

fn suffix_char_pred<F: FnMut(char) -> bool>(hay: &str, f: &mut F) -> Option<usize> {
    if hay.len() == 0 {
        return None;
    }
    let (c, start) = decode_before(hay.as_bytes(), hay.len());
    if f(c) { Some(hay.len() - start) } else { None }
}

impl Pattern for char {
    fn find_in(&mut self, hay: &str, from: usize) -> Option<(usize, usize)> {
        let c = *self;
        if (c as u32) < 128 {
            let b = hay.as_bytes();
            let x = c as u8;
            let mut i = from;
            while i < b.len() {
                if b[i] == x {
                    return Some((i, i + 1));
                }
                i += 1;
            }
            return None;
        }
        find_char_pred(hay, from, &mut |x: char| x == c)
    }
    fn rfind_in(&mut self, hay: &str, to: usize) -> Option<(usize, usize)> {
        let c = *self;
        rfind_char_pred(hay, to, &mut |x: char| x == c)
    }
    fn prefix_len(&mut self, hay: &str) -> Option<usize> {
        let c = *self;
        prefix_char_pred(hay, &mut |x: char| x == c)
    }
    fn suffix_len(&mut self, hay: &str) -> Option<usize> {
        let c = *self;
        suffix_char_pred(hay, &mut |x: char| x == c)
    }
}

fn bytes_eq_at(hay: &[u8], at: usize, needle: &[u8]) -> bool {
    let mut k = 0;
    while k < needle.len() {
        if hay[at + k] != needle[k] {
            return false;
        }
        k += 1;
    }
    true
}

impl Pattern for &str {
    fn find_in(&mut self, hay: &str, from: usize) -> Option<(usize, usize)> {
        let h = hay.as_bytes();
        let n = self.as_bytes();
        if n.len() > h.len() {
            return None;
        }
        let mut i = from;
        while i + n.len() <= h.len() {
            if bytes_eq_at(h, i, n) {
                return Some((i, i + n.len()));
            }
            i += 1;
        }
        None
    }
    fn rfind_in(&mut self, hay: &str, to: usize) -> Option<(usize, usize)> {
        let h = hay.as_bytes();
        let n = self.as_bytes();
        if n.len() > to {
            return None;
        }
        let mut i = to - n.len() + 1;
        while i > 0 {
            i -= 1;
            if bytes_eq_at(h, i, n) {
                return Some((i, i + n.len()));
            }
        }
        None
    }
    fn prefix_len(&mut self, hay: &str) -> Option<usize> {
        let n = self.as_bytes();
        if n.len() <= hay.len() && bytes_eq_at(hay.as_bytes(), 0, n) { Some(n.len()) } else { None }
    }
    fn suffix_len(&mut self, hay: &str) -> Option<usize> {
        let n = self.as_bytes();
        if n.len() <= hay.len() && bytes_eq_at(hay.as_bytes(), hay.len() - n.len(), n) { Some(n.len()) } else { None }
    }
}

impl Pattern for &crate::string::String {
    fn find_in(&mut self, hay: &str, from: usize) -> Option<(usize, usize)> {
        let mut s: &str = self.as_str();
        s.find_in(hay, from)
    }
    fn rfind_in(&mut self, hay: &str, to: usize) -> Option<(usize, usize)> {
        let mut s: &str = self.as_str();
        s.rfind_in(hay, to)
    }
    fn prefix_len(&mut self, hay: &str) -> Option<usize> {
        let mut s: &str = self.as_str();
        s.prefix_len(hay)
    }
    fn suffix_len(&mut self, hay: &str) -> Option<usize> {
        let mut s: &str = self.as_str();
        s.suffix_len(hay)
    }
}

impl Pattern for &&str {
    fn find_in(&mut self, hay: &str, from: usize) -> Option<(usize, usize)> {
        let mut s: &str = **self;
        s.find_in(hay, from)
    }
    fn rfind_in(&mut self, hay: &str, to: usize) -> Option<(usize, usize)> {
        let mut s: &str = **self;
        s.rfind_in(hay, to)
    }
    fn prefix_len(&mut self, hay: &str) -> Option<usize> {
        let mut s: &str = **self;
        s.prefix_len(hay)
    }
    fn suffix_len(&mut self, hay: &str) -> Option<usize> {
        let mut s: &str = **self;
        s.suffix_len(hay)
    }
}

impl<F: FnMut(char) -> bool> Pattern for F {
    fn find_in(&mut self, hay: &str, from: usize) -> Option<(usize, usize)> {
        find_char_pred(hay, from, self)
    }
    fn rfind_in(&mut self, hay: &str, to: usize) -> Option<(usize, usize)> {
        rfind_char_pred(hay, to, self)
    }
    fn prefix_len(&mut self, hay: &str) -> Option<usize> {
        prefix_char_pred(hay, self)
    }
    fn suffix_len(&mut self, hay: &str) -> Option<usize> {
        suffix_char_pred(hay, self)
    }
}

fn in_set(set: &[char], c: char) -> bool {
    let mut i = 0;
    while i < set.len() {
        if set[i] == c {
            return true;
        }
        i += 1;
    }
    false
}

impl<const N: usize> Pattern for [char; N] {
    fn find_in(&mut self, hay: &str, from: usize) -> Option<(usize, usize)> {
        let set: &[char] = self;
        find_char_pred(hay, from, &mut |c: char| in_set(set, c))
    }
    fn rfind_in(&mut self, hay: &str, to: usize) -> Option<(usize, usize)> {
        let set: &[char] = self;
        rfind_char_pred(hay, to, &mut |c: char| in_set(set, c))
    }
    fn prefix_len(&mut self, hay: &str) -> Option<usize> {
        let set: &[char] = self;
        prefix_char_pred(hay, &mut |c: char| in_set(set, c))
    }
    fn suffix_len(&mut self, hay: &str) -> Option<usize> {
        let set: &[char] = self;
        suffix_char_pred(hay, &mut |c: char| in_set(set, c))
    }
}

impl Pattern for &[char] {
    fn find_in(&mut self, hay: &str, from: usize) -> Option<(usize, usize)> {
        let set: &[char] = *self;
        find_char_pred(hay, from, &mut |c: char| in_set(set, c))
    }
    fn rfind_in(&mut self, hay: &str, to: usize) -> Option<(usize, usize)> {
        let set: &[char] = *self;
        rfind_char_pred(hay, to, &mut |c: char| in_set(set, c))
    }
    fn prefix_len(&mut self, hay: &str) -> Option<usize> {
        let set: &[char] = *self;
        prefix_char_pred(hay, &mut |c: char| in_set(set, c))
    }
    fn suffix_len(&mut self, hay: &str) -> Option<usize> {
        let set: &[char] = *self;
        suffix_char_pred(hay, &mut |c: char| in_set(set, c))
    }
}

impl<const N: usize> Pattern for &[char; N] {
    fn find_in(&mut self, hay: &str, from: usize) -> Option<(usize, usize)> {
        let set: &[char] = *self;
        find_char_pred(hay, from, &mut |c: char| in_set(set, c))
    }
    fn rfind_in(&mut self, hay: &str, to: usize) -> Option<(usize, usize)> {
        let set: &[char] = *self;
        rfind_char_pred(hay, to, &mut |c: char| in_set(set, c))
    }
    fn prefix_len(&mut self, hay: &str) -> Option<usize> {
        let set: &[char] = *self;
        prefix_char_pred(hay, &mut |c: char| in_set(set, c))
    }
    fn suffix_len(&mut self, hay: &str) -> Option<usize> {
        let set: &[char] = *self;
        suffix_char_pred(hay, &mut |c: char| in_set(set, c))
    }
}
