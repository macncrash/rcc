#![allow(dead_code)]
// t12: a C lexer in the style of ccc src/frontend/lexer (TokenKind with
// literal-suffix variants, keyword table, maximal-munch punctuators, escape
// sequences, spans with line/column), plus a token-statistics pass.
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Span {
    start: u32,
    end: u32,
    line: u32,
    col: u32,
}

/// Token kinds, modelled on ccc's TokenKind.
#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    IntLiteral(i64),       // no suffix and fits in i32... or larger
    UIntLiteral(u64),      // u/U suffix
    LongLiteral(i64),      // l/L
    ULongLiteral(u64),     // ul/UL
    FloatLiteral(f64),
    FloatLiteralF32(f64),
    StringLiteral(String),
    CharLiteral(char),
    Identifier(String),
    // keywords
    Auto, Break, Case, Char, Const, Continue, Default, Do, Double, Else, Enum, Extern,
    Float, For, Goto, If, Inline, Int, Long, Register, Restrict, Return, Short, Signed,
    Sizeof, Static, Struct, Switch, Typedef, Union, Unsigned, Void, Volatile, While,
    Bool, StaticAssert,
    // punctuators
    LParen, RParen, LBrace, RBrace, LBracket, RBracket, Semicolon, Comma, Dot, Arrow,
    Ellipsis, Plus, Minus, Star, Slash, Percent, Amp, Pipe, Caret, Tilde, Bang,
    Question, Colon, Assign, PlusAssign, MinusAssign, StarAssign, SlashAssign,
    PercentAssign, AmpAssign, PipeAssign, CaretAssign, LessLessAssign,
    GreaterGreaterAssign, PlusPlus, MinusMinus, LessLess, GreaterGreater, Less,
    LessEqual, Greater, GreaterEqual, EqualEqual, BangEqual, AmpAmp, PipePipe, Hash,
    HashHash,
    Eof,
}

impl TokenKind {
    fn keyword(s: &str) -> Option<TokenKind> {
        use TokenKind::*;
        Some(match s {
            "auto" => Auto,
            "break" => Break,
            "case" => Case,
            "char" => Char,
            "const" => Const,
            "continue" => Continue,
            "default" => Default,
            "do" => Do,
            "double" => Double,
            "else" => Else,
            "enum" => Enum,
            "extern" => Extern,
            "float" => Float,
            "for" => For,
            "goto" => Goto,
            "if" => If,
            "inline" | "__inline" | "__inline__" => Inline,
            "int" => Int,
            "long" => Long,
            "register" => Register,
            "restrict" | "__restrict" => Restrict,
            "return" => Return,
            "short" => Short,
            "signed" | "__signed__" => Signed,
            "sizeof" => Sizeof,
            "static" => Static,
            "struct" => Struct,
            "switch" => Switch,
            "typedef" => Typedef,
            "union" => Union,
            "unsigned" => Unsigned,
            "void" => Void,
            "volatile" | "__volatile__" => Volatile,
            "while" => While,
            "_Bool" => Bool,
            "_Static_assert" => StaticAssert,
            _ => return None,
        })
    }

    fn is_keyword(&self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            Auto | Break | Case | Char | Const | Continue | Default | Do | Double | Else | Enum | Extern | Float | For | Goto | If
                | Inline | Int | Long | Register | Restrict | Return | Short | Signed | Sizeof | Static | Struct | Switch
                | Typedef | Union | Unsigned | Void | Volatile | While | Bool | StaticAssert
        )
    }

    fn is_literal(&self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            IntLiteral(_) | UIntLiteral(_) | LongLiteral(_) | ULongLiteral(_) | FloatLiteral(_) | FloatLiteralF32(_) | StringLiteral(_) | CharLiteral(_)
        )
    }
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    span: Span,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{} ", self.span.line, self.span.col)?;
        match &self.kind {
            TokenKind::Identifier(s) => write!(f, "ident {}", s),
            TokenKind::StringLiteral(s) => write!(f, "string {:?}", s),
            TokenKind::CharLiteral(c) => write!(f, "char {:?} ({})", c, *c as u32),
            TokenKind::FloatLiteral(v) => write!(f, "double {:?}", v),
            TokenKind::FloatLiteralF32(v) => write!(f, "float {:?}", v),
            k if k.is_keyword() => write!(f, "kw {:?}", k),
            k => write!(f, "{:?}", k),
        }
    }
}

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: u32,
    line_start: usize,
    errors: Vec<String>,
}

const PUNCT: &[(&str, TokenKind)] = &[
    ("<<=", TokenKind::LessLessAssign),
    (">>=", TokenKind::GreaterGreaterAssign),
    ("...", TokenKind::Ellipsis),
    ("->", TokenKind::Arrow),
    ("++", TokenKind::PlusPlus),
    ("--", TokenKind::MinusMinus),
    ("<<", TokenKind::LessLess),
    (">>", TokenKind::GreaterGreater),
    ("<=", TokenKind::LessEqual),
    (">=", TokenKind::GreaterEqual),
    ("==", TokenKind::EqualEqual),
    ("!=", TokenKind::BangEqual),
    ("&&", TokenKind::AmpAmp),
    ("||", TokenKind::PipePipe),
    ("+=", TokenKind::PlusAssign),
    ("-=", TokenKind::MinusAssign),
    ("*=", TokenKind::StarAssign),
    ("/=", TokenKind::SlashAssign),
    ("%=", TokenKind::PercentAssign),
    ("&=", TokenKind::AmpAssign),
    ("|=", TokenKind::PipeAssign),
    ("^=", TokenKind::CaretAssign),
    ("##", TokenKind::HashHash),
    ("(", TokenKind::LParen),
    (")", TokenKind::RParen),
    ("{", TokenKind::LBrace),
    ("}", TokenKind::RBrace),
    ("[", TokenKind::LBracket),
    ("]", TokenKind::RBracket),
    (";", TokenKind::Semicolon),
    (",", TokenKind::Comma),
    (".", TokenKind::Dot),
    ("+", TokenKind::Plus),
    ("-", TokenKind::Minus),
    ("*", TokenKind::Star),
    ("/", TokenKind::Slash),
    ("%", TokenKind::Percent),
    ("&", TokenKind::Amp),
    ("|", TokenKind::Pipe),
    ("^", TokenKind::Caret),
    ("~", TokenKind::Tilde),
    ("!", TokenKind::Bang),
    ("?", TokenKind::Question),
    (":", TokenKind::Colon),
    ("=", TokenKind::Assign),
    ("<", TokenKind::Less),
    (">", TokenKind::Greater),
    ("#", TokenKind::Hash),
];

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer { src: src.as_bytes(), pos: 0, line: 1, line_start: 0, errors: Vec::new() }
    }

    fn peek_at(&self, off: usize) -> u8 {
        *self.src.get(self.pos + off).unwrap_or(&0)
    }

    fn bump(&mut self) -> u8 {
        let c = self.peek_at(0);
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
            self.line_start = self.pos;
        }
        c
    }

    fn skip_trivia(&mut self) {
        loop {
            match (self.peek_at(0), self.peek_at(1)) {
                (b' ' | b'\t' | b'\r' | b'\n', _) => {
                    self.bump();
                }
                (b'/', b'/') => {
                    while self.peek_at(0) != b'\n' && self.peek_at(0) != 0 {
                        self.bump();
                    }
                }
                (b'/', b'*') => {
                    self.bump();
                    self.bump();
                    loop {
                        if self.peek_at(0) == 0 {
                            self.errors.push(format!("{}: unterminated comment", self.line));
                            return;
                        }
                        if self.peek_at(0) == b'*' && self.peek_at(1) == b'/' {
                            self.bump();
                            self.bump();
                            break;
                        }
                        self.bump();
                    }
                }
                _ => return,
            }
        }
    }

    fn escape(&mut self) -> char {
        // called after the backslash
        let c = self.bump();
        match c {
            b'n' => '\n',
            b't' => '\t',
            b'r' => '\r',
            b'0'..=b'7' => {
                let mut v = (c - b'0') as u32;
                for _ in 0..2 {
                    let d = self.peek_at(0);
                    if (b'0'..=b'7').contains(&d) {
                        v = v * 8 + (d - b'0') as u32;
                        self.bump();
                    } else {
                        break;
                    }
                }
                char::from_u32(v & 0xff).unwrap()
            }
            b'x' => {
                let mut v = 0u32;
                while self.peek_at(0).is_ascii_hexdigit() {
                    v = v * 16 + (self.bump() as char).to_digit(16).unwrap();
                }
                char::from_u32(v & 0xff).unwrap()
            }
            b'a' => '\u{7}',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'v' => '\u{b}',
            b'e' => '\u{1b}',
            b'\\' | b'\'' | b'"' | b'?' => c as char,
            other => {
                self.errors.push(format!("{}: unknown escape \\{}", self.line, other as char));
                other as char
            }
        }
    }

    fn number(&mut self) -> TokenKind {
        let start = self.pos;
        let mut is_float = false;
        let (radix, digits_start) = if self.peek_at(0) == b'0' && (self.peek_at(1) | 0x20) == b'x' {
            self.bump();
            self.bump();
            (16, self.pos)
        } else if self.peek_at(0) == b'0' && (self.peek_at(1) | 0x20) == b'b' {
            self.bump();
            self.bump();
            (2, self.pos)
        } else if self.peek_at(0) == b'0' && self.peek_at(1).is_ascii_digit() {
            (8, self.pos)
        } else {
            (10, self.pos)
        };
        loop {
            let c = self.peek_at(0);
            if c.is_ascii_hexdigit() && radix == 16 || c.is_ascii_digit() {
                self.bump();
            } else if radix == 10 && c == b'.' {
                is_float = true;
                self.bump();
            } else if radix == 10 && (c == b'e' || c == b'E') {
                is_float = true;
                self.bump();
                if self.peek_at(0) == b'+' || self.peek_at(0) == b'-' {
                    self.bump();
                }
            } else {
                break;
            }
        }
        let text = std::str::from_utf8(&self.src[digits_start..self.pos]).unwrap().to_string();
        let mut suffix = String::new();
        while self.peek_at(0).is_ascii_alphabetic() {
            suffix.push((self.bump() as char).to_ascii_lowercase());
        }
        if is_float {
            let v: f64 = text.parse().unwrap_or(0.0);
            return match suffix.as_str() {
                "f" => TokenKind::FloatLiteralF32(v as f32 as f64),
                "" | "l" => TokenKind::FloatLiteral(v),
                _ => {
                    self.errors.push(format!("{}: bad float suffix {}", self.line, suffix));
                    TokenKind::FloatLiteral(v)
                }
            };
        }
        let v = match u64::from_str_radix(&text, radix) {
            Ok(v) => v,
            Err(_) => {
                let whole = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
                self.errors.push(format!("{}: integer literal {} out of range", self.line, whole));
                u64::MAX
            }
        };
        let unsigned = suffix.contains('u');
        let long = suffix.contains('l');
        match (unsigned, long) {
            (true, true) => TokenKind::ULongLiteral(v),
            (true, false) => {
                if v > u32::MAX as u64 { TokenKind::ULongLiteral(v) } else { TokenKind::UIntLiteral(v) }
            }
            (false, true) => {
                if v > i64::MAX as u64 { TokenKind::ULongLiteral(v) } else { TokenKind::LongLiteral(v as i64) }
            }
            (false, false) => {
                if v <= i32::MAX as u64 {
                    TokenKind::IntLiteral(v as i64)
                } else if radix != 10 && v <= u32::MAX as u64 {
                    TokenKind::UIntLiteral(v)
                } else if v <= i64::MAX as u64 {
                    TokenKind::LongLiteral(v as i64)
                } else {
                    TokenKind::ULongLiteral(v)
                }
            }
        }
    }

    fn next_token(&mut self) -> Token {
        self.skip_trivia();
        let start = self.pos;
        let line = self.line;
        let col = (self.pos - self.line_start + 1) as u32;
        let c = self.peek_at(0);
        let kind = if c == 0 {
            TokenKind::Eof
        } else if c.is_ascii_digit() || (c == b'.' && self.peek_at(1).is_ascii_digit()) {
            self.number()
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while self.peek_at(0).is_ascii_alphanumeric() || self.peek_at(0) == b'_' {
                self.bump();
            }
            let w = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
            TokenKind::keyword(w).unwrap_or_else(|| TokenKind::Identifier(w.to_string()))
        } else if c == b'"' {
            self.bump();
            let mut s = String::new();
            loop {
                match self.peek_at(0) {
                    0 | b'\n' => {
                        self.errors.push(format!("{}: unterminated string", line));
                        break;
                    }
                    b'"' => {
                        self.bump();
                        break;
                    }
                    b'\\' => {
                        self.bump();
                        let e = self.escape();
                        s.push(e);
                    }
                    _ => s.push(self.bump() as char),
                }
            }
            TokenKind::StringLiteral(s)
        } else if c == b'\'' {
            self.bump();
            let ch = if self.peek_at(0) == b'\\' {
                self.bump();
                self.escape()
            } else {
                self.bump() as char
            };
            if self.peek_at(0) == b'\'' {
                self.bump();
            } else {
                self.errors.push(format!("{}: bad char literal", line));
            }
            TokenKind::CharLiteral(ch)
        } else {
            let rest = &self.src[self.pos..];
            let mut found = None;
            for (p, k) in PUNCT {
                if rest.starts_with(p.as_bytes()) {
                    found = Some((p.len(), k.clone()));
                    break;
                }
            }
            match found {
                Some((n, k)) => {
                    for _ in 0..n {
                        self.bump();
                    }
                    k
                }
                None => {
                    self.bump();
                    self.errors.push(format!("{}:{}: stray '{}'", line, col, c as char));
                    return self.next_token();
                }
            }
        };
        Token { kind, span: Span { start: start as u32, end: self.pos as u32, line, col } }
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut v = Vec::new();
        loop {
            let t = self.next_token();
            let done = t.kind == TokenKind::Eof;
            v.push(t);
            if done {
                break;
            }
        }
        // adjacent string literal concatenation
        let mut out: Vec<Token> = Vec::with_capacity(v.len());
        for t in v {
            if let (Some(Token { kind: TokenKind::StringLiteral(prev), span }), TokenKind::StringLiteral(s)) = (out.last_mut(), &t.kind) {
                prev.push_str(s);
                span.end = t.span.end;
                continue;
            }
            out.push(t);
        }
        out
    }
}

const SOURCE: &str = r#"/* sample translation unit */
typedef unsigned long size_t;
static inline int max(int a, int b) { return a > b ? a : b; }
struct point { int x, y; };
int main(void) {
    struct point p = { .x = 1, .y = -2 };
    struct point *pp = &p;
    pp->x <<= 3; pp->y >>= 1;
    unsigned u = 0xFFFFFFFFu;
    long big = 1234567890123L;
    unsigned long ul = 18446744073709551615UL;
    int oct = 0755, hex = 0x7f, bin = 0b1011;
    unsigned int wide = 0x80000000;
    long dec_wide = 3000000000;
    double d = 3.14159, e = 1e-3, g = .5;
    float f = 2.5f;
    char c1 = 'a', c2 = '\n', c3 = '\x41', c4 = '\101', c5 = '\\', c6 = '\'';
    const char *s = "hello, " "world\t!" "\x21";
    if (u != 0 && big >= 0 || !oct) { p.x++; --p.y; }
    for (int i = 0; i < 10; i += 2) p.x *= i % 3;
    switch (hex) { case 1: break; default: goto out; }
out:
    return sizeof(struct point) ... ;
}
#define X(a) a ## a
int z = 99999999999999999999;
char bad = '\q';
@
"#;

fn main() {
    let mut lx = Lexer::new(SOURCE);
    let toks = lx.tokenize();
    for t in &toks {
        println!("{}", t);
    }
    println!("--- {} tokens, {} errors", toks.len(), lx.errors.len());
    for e in &lx.errors {
        println!("error: {}", e);
    }
    let mut stats: BTreeMap<&str, usize> = BTreeMap::new();
    for t in &toks {
        let cat = if t.kind.is_keyword() {
            "keyword"
        } else if t.kind.is_literal() {
            "literal"
        } else if matches!(t.kind, TokenKind::Identifier(_)) {
            "identifier"
        } else if t.kind == TokenKind::Eof {
            "eof"
        } else {
            "punct"
        };
        *stats.entry(cat).or_default() += 1;
    }
    println!("stats {:?}", stats);
    let mut idents: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for t in &toks {
        if let TokenKind::Identifier(name) = &t.kind {
            idents.entry(name.clone()).or_default().push(t.span.line);
        }
    }
    for (name, lines) in &idents {
        if lines.len() > 1 {
            println!("ident {} used on lines {:?}", name, lines);
        }
    }
    let longest = toks.iter().max_by_key(|t| (t.span.end - t.span.start, std::cmp::Reverse(t.span.start))).unwrap();
    println!("longest token at {}:{} len {}: {}", longest.span.line, longest.span.col, longest.span.end - longest.span.start, longest);
    let sum_ints: i128 = toks
        .iter()
        .map(|t| match t.kind {
            TokenKind::IntLiteral(v) | TokenKind::LongLiteral(v) => v as i128,
            TokenKind::UIntLiteral(v) | TokenKind::ULongLiteral(v) => v as i128,
            _ => 0,
        })
        .sum();
    println!("sum of integer literals {}", sum_ints);
}
