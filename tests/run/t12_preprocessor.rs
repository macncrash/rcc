#![allow(dead_code)]
// t12: a mini C preprocessor: line splicing, comment stripping, object- and
// function-like macros with # and ##, recursion suppression, #undef,
// #ifdef/#ifndef/#if/#elif/#else/#endif with a constant-expression evaluator
// (defined(), arithmetic, logic, ternary), #include from an in-memory file
// table with include guards, #error, __LINE__/__FILE__.
// Byte helpers and parse_macro_args lifted (lightly adapted) from ccc
// src/frontend/preprocessor/utils.rs and macro_defs.rs.
use std::collections::{HashMap, HashSet};

// ---- lifted from ccc src/frontend/preprocessor/utils.rs ---------------------
#[inline(always)]
pub fn is_ident_start_byte(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}
#[inline(always)]
pub fn is_ident_cont_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}
#[inline(always)]
pub fn bytes_to_str(bytes: &[u8], start: usize, end: usize) -> &str {
    std::str::from_utf8(&bytes[start..end]).expect("bytes_to_str: input is not valid UTF-8")
}
pub fn skip_literal_bytes(bytes: &[u8], start: usize, quote: u8) -> usize {
    let len = bytes.len();
    let mut i = start + 1; // skip opening quote
    while i < len {
        if bytes[i] == b'\\' && i + 1 < len {
            i += 2;
        } else if bytes[i] == quote {
            return i + 1;
        } else {
            i += 1;
        }
    }
    i
}

// ---- lifted from ccc src/frontend/preprocessor/macro_defs.rs ----------------
fn parse_macro_args(bytes: &[u8], start: usize) -> (Vec<String>, usize) {
    let len = bytes.len();
    let mut i = start + 1; // skip '('
    let mut args: Vec<String> = Vec::new();
    let mut paren_depth = 0;
    let mut arg_start = i;
    while i < len {
        match bytes[i] {
            b'(' => {
                paren_depth += 1;
                i += 1;
            }
            b')' => {
                if paren_depth == 0 {
                    let arg_slice = std::str::from_utf8(&bytes[arg_start..i]).unwrap_or("");
                    let trimmed = arg_slice.trim();
                    if !trimmed.is_empty() || !args.is_empty() {
                        args.push(trimmed.to_string());
                    }
                    return (args, i + 1);
                } else {
                    paren_depth -= 1;
                    i += 1;
                }
            }
            b',' if paren_depth == 0 => {
                let arg_slice = std::str::from_utf8(&bytes[arg_start..i]).unwrap_or("");
                args.push(arg_slice.trim().to_string());
                i += 1;
                arg_start = i;
            }
            b'"' | b'\'' => {
                i = skip_literal_bytes(bytes, i, bytes[i]);
            }
            _ => {
                i += 1;
            }
        }
    }
    let arg_slice = std::str::from_utf8(&bytes[arg_start..i]).unwrap_or("");
    let trimmed = arg_slice.trim();
    if !trimmed.is_empty() || !args.is_empty() {
        args.push(trimmed.to_string());
    }
    (args, i)
}

fn stringify_arg(arg: &str) -> String {
    let mut out = String::from("\"");
    let mut last_space = false;
    for c in arg.trim().chars() {
        if c.is_whitespace() {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
            continue;
        }
        last_space = false;
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

// ---- macro table -------------------------------------------------------------

#[derive(Debug, Clone)]
struct MacroDef {
    name: String,
    params: Option<Vec<String>>, // None = object-like
    variadic: bool,
    body: String,
}

fn parse_define(rest: &str) -> Option<MacroDef> {
    let b = rest.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i] == b' ' {
        i += 1;
    }
    let s = i;
    if i >= b.len() || !is_ident_start_byte(b[i]) {
        return None;
    }
    while i < b.len() && is_ident_cont_byte(b[i]) {
        i += 1;
    }
    let name = bytes_to_str(b, s, i).to_string();
    if i < b.len() && b[i] == b'(' {
        let close = rest[i..].find(')')? + i;
        let mut params: Vec<String> = rest[i + 1..close].split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
        let mut variadic = false;
        if params.last().map(|p| p == "...").unwrap_or(false) {
            params.pop();
            variadic = true;
        }
        Some(MacroDef { name, params: Some(params), variadic, body: rest[close + 1..].trim().to_string() })
    } else {
        Some(MacroDef { name, params: None, variadic: false, body: rest[i..].trim().to_string() })
    }
}

struct Preprocessor {
    macros: HashMap<String, MacroDef>,
    files: HashMap<&'static str, &'static str>,
    out: Vec<String>,
    errors: Vec<String>,
    included: Vec<String>,
    expansions: usize,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum CondState {
    Active,
    Inactive,   // looking for a true branch
    Done,       // a branch was taken; skip the rest
    Skipping,   // nested inside an inactive region
}

impl Preprocessor {
    fn new() -> Self {
        Preprocessor { macros: HashMap::new(), files: HashMap::new(), out: Vec::new(), errors: Vec::new(), included: Vec::new(), expansions: 0 }
    }

    fn expand(&mut self, text: &str, hide: &mut HashSet<String>, line: usize, file: &str) -> String {
        let bytes = text.as_bytes();
        let mut result = String::new();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            if c == b'"' || c == b'\'' {
                let e = skip_literal_bytes(bytes, i, c);
                result.push_str(bytes_to_str(bytes, i, e));
                i = e;
                continue;
            }
            if c.is_ascii_digit() {
                let s = i;
                while i < bytes.len() && (is_ident_cont_byte(bytes[i]) || bytes[i] == b'.') {
                    i += 1;
                }
                result.push_str(bytes_to_str(bytes, s, i));
                continue;
            }
            if !is_ident_start_byte(c) {
                result.push(c as char);
                i += 1;
                continue;
            }
            let s = i;
            while i < bytes.len() && is_ident_cont_byte(bytes[i]) {
                i += 1;
            }
            let ident = bytes_to_str(bytes, s, i).to_string();
            if ident == "__LINE__" {
                result.push_str(&line.to_string());
                continue;
            }
            if ident == "__FILE__" {
                result.push_str(&format!("\"{}\"", file));
                continue;
            }
            if hide.contains(&ident) {
                result.push_str(&ident);
                continue;
            }
            let Some(def) = self.macros.get(&ident).cloned() else {
                result.push_str(&ident);
                continue;
            };
            match &def.params {
                None => {
                    self.expansions += 1;
                    hide.insert(ident.clone());
                    let e = self.expand(&def.body, hide, line, file);
                    hide.remove(&ident);
                    result.push_str(&e);
                }
                Some(params) => {
                    let mut j = i;
                    while j < bytes.len() && bytes[j] == b' ' {
                        j += 1;
                    }
                    if j >= bytes.len() || bytes[j] != b'(' {
                        result.push_str(&ident);
                        continue;
                    }
                    let (args, end) = parse_macro_args(bytes, j);
                    i = end;
                    self.expansions += 1;
                    let substituted = self.substitute(&def, params, &args, hide, line, file);
                    hide.insert(ident.clone());
                    let e = self.expand(&substituted, hide, line, file);
                    hide.remove(&ident);
                    result.push_str(&e);
                }
            }
        }
        result
    }

    fn substitute(&mut self, def: &MacroDef, params: &[String], args: &[String], hide: &mut HashSet<String>, line: usize, file: &str) -> String {
        let lookup = |name: &str| -> Option<String> {
            if name == "__VA_ARGS__" && def.variadic {
                return Some(args.get(params.len()..).map(|r| r.join(", ")).unwrap_or_default());
            }
            params.iter().position(|p| p == name).map(|k| args.get(k).cloned().unwrap_or_default())
        };
        // tokenize body into (text, is_ident)
        let b = def.body.as_bytes();
        let mut toks: Vec<(String, bool)> = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if is_ident_start_byte(b[i]) {
                let s = i;
                while i < b.len() && is_ident_cont_byte(b[i]) {
                    i += 1;
                }
                toks.push((bytes_to_str(b, s, i).to_string(), true));
            } else if b[i] == b'#' && i + 1 < b.len() && b[i + 1] == b'#' {
                toks.push(("##".to_string(), false));
                i += 2;
            } else if b[i] == b'"' {
                let e = skip_literal_bytes(b, i, b'"');
                toks.push((bytes_to_str(b, i, e).to_string(), false));
                i = e;
            } else {
                toks.push(((b[i] as char).to_string(), false));
                i += 1;
            }
        }
        let mut out = String::new();
        let mut k = 0;
        while k < toks.len() {
            let (t, is_id) = &toks[k];
            if t == "#" {
                // stringify next identifier param
                let mut n = k + 1;
                while n < toks.len() && toks[n].0 == " " {
                    n += 1;
                }
                if n < toks.len() {
                    if let Some(a) = lookup(&toks[n].0) {
                        out.push_str(&stringify_arg(&a));
                        k = n + 1;
                        continue;
                    }
                }
                out.push('#');
                k += 1;
                continue;
            }
            if t == "##" {
                while out.ends_with(' ') {
                    out.pop();
                }
                let mut n = k + 1;
                while n < toks.len() && toks[n].0 == " " {
                    n += 1;
                }
                if n < toks.len() {
                    let raw = if toks[n].1 { lookup(&toks[n].0).unwrap_or_else(|| toks[n].0.clone()) } else { toks[n].0.clone() };
                    out.push_str(&raw);
                }
                k = n + 1;
                continue;
            }
            if *is_id {
                if let Some(a) = lookup(t) {
                    // pasted operands are not pre-expanded
                    let next_is_paste = toks[k + 1..].iter().find(|x| x.0 != " ").map(|x| x.0 == "##").unwrap_or(false);
                    if next_is_paste {
                        out.push_str(&a);
                    } else {
                        let e = self.expand(&a, hide, line, file);
                        out.push_str(&e);
                    }
                    k += 1;
                    continue;
                }
            }
            out.push_str(t);
            k += 1;
        }
        out
    }

    fn eval_cond(&mut self, expr: &str, line: usize, file: &str) -> i64 {
        // replace defined(X) / defined X first
        let b = expr.as_bytes();
        let mut pre = String::new();
        let mut i = 0;
        while i < b.len() {
            if is_ident_start_byte(b[i]) {
                let s = i;
                while i < b.len() && is_ident_cont_byte(b[i]) {
                    i += 1;
                }
                let id = bytes_to_str(b, s, i);
                if id == "defined" {
                    while i < b.len() && (b[i] == b' ' || b[i] == b'(') {
                        i += 1;
                    }
                    let s2 = i;
                    while i < b.len() && is_ident_cont_byte(b[i]) {
                        i += 1;
                    }
                    let name = bytes_to_str(b, s2, i);
                    while i < b.len() && (b[i] == b' ' || b[i] == b')') {
                        i += 1;
                    }
                    pre.push_str(if self.macros.contains_key(name) { " 1 " } else { " 0 " });
                } else {
                    pre.push_str(id);
                }
            } else {
                pre.push(b[i] as char);
                i += 1;
            }
        }
        let expanded = self.expand(&pre, &mut HashSet::new(), line, file);
        let toks = cond_tokens(&expanded);
        let mut p = CondParser { toks, pos: 0 };
        p.ternary()
    }

    fn run_file(&mut self, name: &str, depth: usize) {
        let Some(src) = self.files.get(name).copied() else {
            self.errors.push(format!("{}: file not found", name));
            return;
        };
        if depth > 8 {
            self.errors.push(format!("{}: include depth exceeded", name));
            return;
        }
        self.included.push(format!("{}{}", "  ".repeat(depth), name));
        let lines = splice_and_strip(src);
        let mut stack: Vec<CondState> = Vec::new();
        for (lineno, raw) in lines {
            let trimmed = raw.trim_start();
            let active = stack.iter().all(|s| *s == CondState::Active);
            if let Some(directive) = trimmed.strip_prefix('#') {
                let directive = directive.trim_start();
                let (kw, rest) = match directive.find(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
                    Some(p) => (&directive[..p], directive[p..].trim()),
                    None => (directive, ""),
                };
                match kw {
                    "ifdef" | "ifndef" | "if" => {
                        if !active {
                            stack.push(CondState::Skipping);
                            continue;
                        }
                        let v = match kw {
                            "ifdef" => self.macros.contains_key(rest),
                            "ifndef" => !self.macros.contains_key(rest),
                            _ => self.eval_cond(rest, lineno, name) != 0,
                        };
                        stack.push(if v { CondState::Active } else { CondState::Inactive });
                    }
                    "elif" => {
                        let top = stack.last().copied();
                        let new = match top {
                            Some(CondState::Active) => CondState::Done,
                            Some(CondState::Inactive) => {
                                if self.eval_cond(rest, lineno, name) != 0 { CondState::Active } else { CondState::Inactive }
                            }
                            Some(s) => s,
                            None => {
                                self.errors.push(format!("{}:{}: #elif without #if", name, lineno));
                                continue;
                            }
                        };
                        *stack.last_mut().unwrap() = new;
                    }
                    "else" => match stack.last_mut() {
                        Some(s) => {
                            *s = match *s {
                                CondState::Active => CondState::Done,
                                CondState::Inactive => CondState::Active,
                                other => other,
                            }
                        }
                        None => self.errors.push(format!("{}:{}: #else without #if", name, lineno)),
                    },
                    "endif" => {
                        if stack.pop().is_none() {
                            self.errors.push(format!("{}:{}: #endif without #if", name, lineno));
                        }
                    }
                    _ if !active => {}
                    "define" => match parse_define(rest) {
                        Some(def) => {
                            if let Some(old) = self.macros.get(&def.name) {
                                if old.body != def.body {
                                    self.errors.push(format!("{}:{}: warning: {} redefined", name, lineno, def.name));
                                }
                            }
                            self.macros.insert(def.name.clone(), def);
                        }
                        None => self.errors.push(format!("{}:{}: bad #define", name, lineno)),
                    },
                    "undef" => {
                        self.macros.remove(rest);
                    }
                    "include" => {
                        let inner = rest.trim_matches(|c| c == '"' || c == '<' || c == '>');
                        self.run_file(inner, depth + 1);
                    }
                    "error" => self.errors.push(format!("{}:{}: #error {}", name, lineno, rest)),
                    "" => {}
                    other => self.errors.push(format!("{}:{}: unknown directive #{}", name, lineno, other)),
                }
                continue;
            }
            if !active {
                continue;
            }
            let e = self.expand(&raw, &mut HashSet::new(), lineno, name);
            let e = collapse_ws(&e);
            if !e.is_empty() {
                self.out.push(e);
            }
        }
        if !stack.is_empty() {
            self.errors.push(format!("{}: unterminated conditional ({} open)", name, stack.len()));
        }
    }
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Join backslash-continued lines and strip comments, keeping the original
/// line number of the first physical line.
fn splice_and_strip(src: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut start_line = 0;
    let mut in_block = false;
    for (n, line) in src.lines().enumerate() {
        if cur.is_empty() {
            start_line = n + 1;
        }
        let (body, cont) = match line.strip_suffix('\\') {
            Some(b) => (b, true),
            None => (line, false),
        };
        // strip comments
        let bytes = body.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if in_block {
                if bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    in_block = false;
                    i += 2;
                    cur.push(' ');
                } else {
                    i += 1;
                }
                continue;
            }
            match bytes[i] {
                b'"' | b'\'' => {
                    let e = skip_literal_bytes(bytes, i, bytes[i]);
                    cur.push_str(bytes_to_str(bytes, i, e));
                    i = e;
                }
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => break,
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                    in_block = true;
                    i += 2;
                }
                c => {
                    cur.push(c as char);
                    i += 1;
                }
            }
        }
        if cont || in_block {
            continue;
        }
        out.push((start_line, std::mem::take(&mut cur)));
    }
    if !cur.is_empty() {
        out.push((start_line, cur));
    }
    out
}

// ---- #if expression evaluator --------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum CTok {
    Num(i64),
    Op(&'static str),
}

fn cond_tokens(s: &str) -> Vec<CTok> {
    const OPS: [&str; 22] = ["||", "&&", "==", "!=", "<=", ">=", "<<", ">>", "<", ">", "+", "-", "*", "/", "%", "!", "~", "(", ")", "?", ":", "&"];
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    'outer: while i < b.len() {
        if b[i] == b' ' {
            i += 1;
            continue;
        }
        if b[i].is_ascii_digit() {
            let s0 = i;
            while i < b.len() && b[i].is_ascii_alphanumeric() {
                i += 1;
            }
            let lit = bytes_to_str(b, s0, i).trim_end_matches(|c| c == 'u' || c == 'U' || c == 'l' || c == 'L');
            let v = if let Some(h) = lit.strip_prefix("0x") { i64::from_str_radix(h, 16).unwrap_or(0) } else { lit.parse().unwrap_or(0) };
            out.push(CTok::Num(v));
            continue;
        }
        if is_ident_start_byte(b[i]) {
            // unknown identifiers evaluate to 0
            while i < b.len() && is_ident_cont_byte(b[i]) {
                i += 1;
            }
            out.push(CTok::Num(0));
            continue;
        }
        for op in OPS.iter() {
            if s[i..].starts_with(op) {
                out.push(CTok::Op(op));
                i += op.len();
                continue 'outer;
            }
        }
        if b[i] == b'|' {
            out.push(CTok::Op("|"));
        } else if b[i] == b'^' {
            out.push(CTok::Op("^"));
        }
        i += 1;
    }
    out
}

struct CondParser {
    toks: Vec<CTok>,
    pos: usize,
}

impl CondParser {
    fn peek_op(&self) -> Option<&'static str> {
        match self.toks.get(self.pos) {
            Some(CTok::Op(o)) => Some(o),
            _ => None,
        }
    }
    fn ternary(&mut self) -> i64 {
        let c = self.binary(0);
        if self.peek_op() == Some("?") {
            self.pos += 1;
            let a = self.ternary();
            if self.peek_op() == Some(":") {
                self.pos += 1;
            }
            let b = self.ternary();
            return if c != 0 { a } else { b };
        }
        c
    }
    fn prec(op: &str) -> Option<u8> {
        Some(match op {
            "||" => 1,
            "&&" => 2,
            "|" => 3,
            "^" => 4,
            "&" => 5,
            "==" | "!=" => 6,
            "<" | ">" | "<=" | ">=" => 7,
            "<<" | ">>" => 8,
            "+" | "-" => 9,
            "*" | "/" | "%" => 10,
            _ => return None,
        })
    }
    fn binary(&mut self, min: u8) -> i64 {
        let mut l = self.unary();
        while let Some(op) = self.peek_op() {
            let Some(p) = Self::prec(op) else { break };
            if p < min {
                break;
            }
            self.pos += 1;
            let r = self.binary(p + 1);
            l = match op {
                "||" => ((l != 0) || (r != 0)) as i64,
                "&&" => ((l != 0) && (r != 0)) as i64,
                "|" => l | r,
                "^" => l ^ r,
                "&" => l & r,
                "==" => (l == r) as i64,
                "!=" => (l != r) as i64,
                "<" => (l < r) as i64,
                ">" => (l > r) as i64,
                "<=" => (l <= r) as i64,
                ">=" => (l >= r) as i64,
                "<<" => l.wrapping_shl(r as u32),
                ">>" => l.wrapping_shr(r as u32),
                "+" => l.wrapping_add(r),
                "-" => l.wrapping_sub(r),
                "*" => l.wrapping_mul(r),
                "/" => if r == 0 { 0 } else { l.wrapping_div(r) },
                "%" => if r == 0 { 0 } else { l.wrapping_rem(r) },
                _ => unreachable!(),
            };
        }
        l
    }
    fn unary(&mut self) -> i64 {
        match self.toks.get(self.pos).cloned() {
            Some(CTok::Op("!")) => {
                self.pos += 1;
                (self.unary() == 0) as i64
            }
            Some(CTok::Op("-")) => {
                self.pos += 1;
                self.unary().wrapping_neg()
            }
            Some(CTok::Op("~")) => {
                self.pos += 1;
                !self.unary()
            }
            Some(CTok::Op("(")) => {
                self.pos += 1;
                let v = self.ternary();
                if self.peek_op() == Some(")") {
                    self.pos += 1;
                }
                v
            }
            Some(CTok::Num(n)) => {
                self.pos += 1;
                n
            }
            _ => {
                self.pos += 1;
                0
            }
        }
    }
}

const CONFIG_H: &str = r#"#ifndef CONFIG_H
#define CONFIG_H
#define VERSION_MAJOR 2
#define VERSION_MINOR 7
#define VERSION ((VERSION_MAJOR << 8) | VERSION_MINOR)
#define ARCH_X86 1
#define FEATURE_FAST
#include "limits.h"
#endif
"#;

const LIMITS_H: &str = r#"#ifndef LIMITS_H
#define LIMITS_H
#define INT_MAX 2147483647
#define CHAR_BIT 8
#endif
"#;

const MAIN_C: &str = r#"/* main translation unit */
#include "config.h"
#include "config.h"
#define STR(x) #x
#define XSTR(x) STR(x)
#define CAT(a, b) a ## b
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define SQUARE(x) ((x) * (x))
#define LOG(fmt, ...) printf(fmt, __VA_ARGS__)
#define SELF SELF + 1
#define LONG_MACRO(a, b) \
    do { \
        int tmp = (a); \
        (a) = (b); (b) = tmp; \
    } while (0)
int version = VERSION; // trailing comment
const char *s = STR(hello   "world"  \n);
const char *xs = XSTR(VERSION_MINOR);
int CAT(var, 42) = MAX(SQUARE(3), INT_MAX / CHAR_BIT);
int self = SELF;
LONG_MACRO(x, y);
LOG("%d %d\n", 1, MAX(2, 3));
int line = __LINE__; const char *f = __FILE__;
const char *lit = "MAX(1,2) is not expanded in strings";
#if VERSION >= 0x200 && defined(ARCH_X86)
int arch = 86;
#elif defined ARCH_ARM
int arch = 64;
#else
int arch = 0;
#endif
#ifdef FEATURE_SLOW
int slow = 1;
#  ifdef NESTED
int never = 1;
#  endif
#else
int fast = 1;
#endif
#if (1 ? 2 : 3) == 2 && !defined(UNDEFINED_THING) && (7 % 4 == 3) && (-1 < 0)
int ternary_ok = 1;
#endif
#if 0
#error this must not fire
#endif
#undef MAX
int m = MAX(1, 2);
#define MAX 99
int m2 = MAX;
#if CHAR_BIT * 4 == 32 && INT_MAX > 0x7ffffffe
int bits = 32;
#endif
#if 1
#if 0
int a = 1;
#elif 1
int b = 2;
#elif 1
int c = 3;
#else
int d = 4;
#endif
#endif
#bogus directive
#error stop here
#include "missing.h"
/* multi-line
   comment */ int after_comment = 1;
"#;

fn main() {
    let mut pp = Preprocessor::new();
    pp.files.insert("config.h", CONFIG_H);
    pp.files.insert("limits.h", LIMITS_H);
    pp.files.insert("main.c", MAIN_C);
    pp.run_file("main.c", 0);
    println!("--- output ({} lines)", pp.out.len());
    for l in &pp.out {
        println!("{}", l);
    }
    println!("--- includes");
    for i in &pp.included {
        println!("{}", i);
    }
    println!("--- diagnostics");
    for e in &pp.errors {
        println!("{}", e);
    }
    let mut names: Vec<&String> = pp.macros.keys().collect();
    names.sort();
    println!("--- {} macros defined, {} expansions", names.len(), pp.expansions);
    for n in names {
        let d = &pp.macros[n];
        match &d.params {
            Some(p) => println!("{}({}{}) = {}", n, p.join(","), if d.variadic { ",..." } else { "" }, d.body),
            None => println!("{} = {}", n, d.body),
        }
    }
    // standalone evaluator checks
    for e in ["1 + 2 * 3", "(1 + 2) * 3", "10 / 3 - 10 % 3", "1 << 4 | 1", "~0 & 0xff", "5 > 3 ? 10 : 20", "!0 + !5", "0x10 == 16", "3 - -3", "100 / 0"] {
        println!("#if {} => {}", e, pp.eval_cond(e, 0, "expr"));
    }
}
