// t12: tokenizer + recursive-descent parser + tree-walking evaluator for a
// small expression/statement language with variables, functions, if/while.
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(i64),
    Ident(String),
    Str(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semi,
    Assign,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Bang,
    KwLet,
    KwFn,
    KwIf,
    KwElse,
    KwWhile,
    KwReturn,
    KwPrint,
    Eof,
}

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: u32,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer { src: src.as_bytes(), pos: 0, line: 1 }
    }

    fn peek(&self) -> u8 {
        if self.pos < self.src.len() { self.src[self.pos] } else { 0 }
    }

    fn peek2(&self) -> u8 {
        if self.pos + 1 < self.src.len() { self.src[self.pos + 1] } else { 0 }
    }

    fn skip_ws(&mut self) {
        loop {
            let c = self.peek();
            if c == b'\n' {
                self.line += 1;
                self.pos += 1;
            } else if c == b' ' || c == b'\t' || c == b'\r' {
                self.pos += 1;
            } else if c == b'/' && self.peek2() == b'/' {
                while self.peek() != b'\n' && self.peek() != 0 {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn next_tok(&mut self) -> Result<(Tok, u32), String> {
        self.skip_ws();
        let line = self.line;
        let c = self.peek();
        if c == 0 {
            return Ok((Tok::Eof, line));
        }
        if c.is_ascii_digit() {
            let mut v: i64 = 0;
            while self.peek().is_ascii_digit() {
                v = v * 10 + (self.peek() - b'0') as i64;
                self.pos += 1;
            }
            return Ok((Tok::Num(v), line));
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            let start = self.pos;
            while self.peek().is_ascii_alphanumeric() || self.peek() == b'_' {
                self.pos += 1;
            }
            let word = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
            let t = match word {
                "let" => Tok::KwLet,
                "fn" => Tok::KwFn,
                "if" => Tok::KwIf,
                "else" => Tok::KwElse,
                "while" => Tok::KwWhile,
                "return" => Tok::KwReturn,
                "print" => Tok::KwPrint,
                _ => Tok::Ident(word.to_string()),
            };
            return Ok((t, line));
        }
        if c == b'"' {
            self.pos += 1;
            let mut s = String::new();
            while self.peek() != b'"' {
                if self.peek() == 0 {
                    return Err(format!("line {}: unterminated string", line));
                }
                s.push(self.peek() as char);
                self.pos += 1;
            }
            self.pos += 1;
            return Ok((Tok::Str(s), line));
        }
        let two = (c, self.peek2());
        let (t, n) = match two {
            (b'=', b'=') => (Tok::EqEq, 2),
            (b'!', b'=') => (Tok::NotEq, 2),
            (b'<', b'=') => (Tok::Le, 2),
            (b'>', b'=') => (Tok::Ge, 2),
            (b'&', b'&') => (Tok::AndAnd, 2),
            (b'|', b'|') => (Tok::OrOr, 2),
            (b'+', _) => (Tok::Plus, 1),
            (b'-', _) => (Tok::Minus, 1),
            (b'*', _) => (Tok::Star, 1),
            (b'/', _) => (Tok::Slash, 1),
            (b'%', _) => (Tok::Percent, 1),
            (b'(', _) => (Tok::LParen, 1),
            (b')', _) => (Tok::RParen, 1),
            (b'{', _) => (Tok::LBrace, 1),
            (b'}', _) => (Tok::RBrace, 1),
            (b',', _) => (Tok::Comma, 1),
            (b';', _) => (Tok::Semi, 1),
            (b'=', _) => (Tok::Assign, 1),
            (b'<', _) => (Tok::Lt, 1),
            (b'>', _) => (Tok::Gt, 1),
            (b'!', _) => (Tok::Bang, 1),
            _ => return Err(format!("line {}: unexpected character '{}'", line, c as char)),
        };
        self.pos += n;
        Ok((t, line))
    }

    fn tokenize(mut self) -> Result<Vec<(Tok, u32)>, String> {
        let mut out = Vec::new();
        loop {
            let t = self.next_tok()?;
            let eof = t.0 == Tok::Eof;
            out.push(t);
            if eof {
                break;
            }
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinOp {
    fn sym(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        }
    }
}

#[derive(Debug, Clone)]
enum Expr {
    Num(i64),
    Var(String),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Num(n) => write!(f, "{}", n),
            Expr::Var(v) => write!(f, "{}", v),
            Expr::Neg(e) => write!(f, "(-{})", e),
            Expr::Not(e) => write!(f, "(!{})", e),
            Expr::Bin { op, lhs, rhs } => write!(f, "({} {} {})", lhs, op.sym(), rhs),
            Expr::Call { name, args } => {
                write!(f, "{}(", name)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ")")
            }
        }
    }
}

#[derive(Debug, Clone)]
enum Stmt {
    Let { name: String, init: Expr },
    Assign { name: String, value: Expr },
    Print { label: Option<String>, value: Expr },
    If { cond: Expr, then_body: Vec<Stmt>, else_body: Vec<Stmt> },
    While { cond: Expr, body: Vec<Stmt> },
    Return(Expr),
    ExprStmt(Expr),
}

#[derive(Debug, Clone)]
struct FnDef {
    name: String,
    params: Vec<String>,
    body: Vec<Stmt>,
}

struct Parser {
    toks: Vec<(Tok, u32)>,
    pos: usize,
}

type PResult<T> = Result<T, String>;

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].0
    }

    fn line(&self) -> u32 {
        self.toks[self.pos].1
    }

    fn advance(&mut self) -> Tok {
        let t = self.toks[self.pos].0.clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, t: Tok) -> PResult<()> {
        if *self.peek() == t {
            self.advance();
            Ok(())
        } else {
            Err(format!("line {}: expected {:?}, found {:?}", self.line(), t, self.peek()))
        }
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == t {
            self.advance();
            true
        } else {
            false
        }
    }

    fn ident(&mut self) -> PResult<String> {
        match self.advance() {
            Tok::Ident(s) => Ok(s),
            other => Err(format!("line {}: expected identifier, found {:?}", self.line(), other)),
        }
    }

    fn program(&mut self) -> PResult<(Vec<FnDef>, Vec<Stmt>)> {
        let mut fns = Vec::new();
        let mut stmts = Vec::new();
        while *self.peek() != Tok::Eof {
            if *self.peek() == Tok::KwFn {
                fns.push(self.fn_def()?);
            } else {
                stmts.push(self.stmt()?);
            }
        }
        Ok((fns, stmts))
    }

    fn fn_def(&mut self) -> PResult<FnDef> {
        self.expect(Tok::KwFn)?;
        let name = self.ident()?;
        self.expect(Tok::LParen)?;
        let mut params = Vec::new();
        if *self.peek() != Tok::RParen {
            loop {
                params.push(self.ident()?);
                if !self.eat(&Tok::Comma) {
                    break;
                }
            }
        }
        self.expect(Tok::RParen)?;
        let body = self.block()?;
        Ok(FnDef { name, params, body })
    }

    fn block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect(Tok::LBrace)?;
        let mut v = Vec::new();
        while *self.peek() != Tok::RBrace {
            v.push(self.stmt()?);
        }
        self.expect(Tok::RBrace)?;
        Ok(v)
    }

    fn stmt(&mut self) -> PResult<Stmt> {
        match self.peek().clone() {
            Tok::KwLet => {
                self.advance();
                let name = self.ident()?;
                self.expect(Tok::Assign)?;
                let init = self.expr()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Let { name, init })
            }
            Tok::KwPrint => {
                self.advance();
                let label = if let Tok::Str(s) = self.peek().clone() {
                    self.advance();
                    self.expect(Tok::Comma)?;
                    Some(s)
                } else {
                    None
                };
                let value = self.expr()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Print { label, value })
            }
            Tok::KwIf => {
                self.advance();
                let cond = self.expr()?;
                let then_body = self.block()?;
                let else_body = if self.eat(&Tok::KwElse) {
                    if *self.peek() == Tok::KwIf {
                        vec![self.stmt()?]
                    } else {
                        self.block()?
                    }
                } else {
                    Vec::new()
                };
                Ok(Stmt::If { cond, then_body, else_body })
            }
            Tok::KwWhile => {
                self.advance();
                let cond = self.expr()?;
                let body = self.block()?;
                Ok(Stmt::While { cond, body })
            }
            Tok::KwReturn => {
                self.advance();
                let e = self.expr()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Return(e))
            }
            Tok::Ident(name) if self.toks[self.pos + 1].0 == Tok::Assign => {
                self.advance();
                self.advance();
                let value = self.expr()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Assign { name, value })
            }
            _ => {
                let e = self.expr()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::ExprStmt(e))
            }
        }
    }

    fn expr(&mut self) -> PResult<Expr> {
        self.binary(0)
    }

    fn binop_info(t: &Tok) -> Option<(BinOp, u8)> {
        Some(match t {
            Tok::OrOr => (BinOp::Or, 1),
            Tok::AndAnd => (BinOp::And, 2),
            Tok::EqEq => (BinOp::Eq, 3),
            Tok::NotEq => (BinOp::Ne, 3),
            Tok::Lt => (BinOp::Lt, 4),
            Tok::Le => (BinOp::Le, 4),
            Tok::Gt => (BinOp::Gt, 4),
            Tok::Ge => (BinOp::Ge, 4),
            Tok::Plus => (BinOp::Add, 5),
            Tok::Minus => (BinOp::Sub, 5),
            Tok::Star => (BinOp::Mul, 6),
            Tok::Slash => (BinOp::Div, 6),
            Tok::Percent => (BinOp::Rem, 6),
            _ => return None,
        })
    }

    // precedence climbing
    fn binary(&mut self, min_prec: u8) -> PResult<Expr> {
        let mut lhs = self.unary()?;
        while let Some((op, prec)) = Self::binop_info(self.peek()) {
            if prec < min_prec {
                break;
            }
            self.advance();
            let rhs = self.binary(prec + 1)?;
            lhs = Expr::Bin { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> PResult<Expr> {
        if self.eat(&Tok::Minus) {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat(&Tok::Bang) {
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        self.primary()
    }

    fn primary(&mut self) -> PResult<Expr> {
        let line = self.line();
        match self.advance() {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Ident(name) => {
                if self.eat(&Tok::LParen) {
                    let mut args = Vec::new();
                    if *self.peek() != Tok::RParen {
                        loop {
                            args.push(self.expr()?);
                            if !self.eat(&Tok::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(Tok::RParen)?;
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Var(name))
                }
            }
            Tok::LParen => {
                let e = self.expr()?;
                self.expect(Tok::RParen)?;
                Ok(e)
            }
            t => Err(format!("line {}: unexpected token {:?}", line, t)),
        }
    }
}

enum Flow {
    Normal,
    Return(i64),
}

struct Interp {
    fns: HashMap<String, FnDef>,
    frames: Vec<HashMap<String, i64>>,
    calls: u64,
    output: Vec<String>,
}

impl Interp {
    fn lookup(&self, name: &str) -> Result<i64, String> {
        let frame = self.frames.last().unwrap();
        frame.get(name).copied().ok_or_else(|| format!("undefined variable {}", name))
    }

    fn eval(&mut self, e: &Expr) -> Result<i64, String> {
        Ok(match e {
            Expr::Num(n) => *n,
            Expr::Var(v) => self.lookup(v)?,
            Expr::Neg(inner) => self.eval(inner)?.wrapping_neg(),
            Expr::Not(inner) => (self.eval(inner)? == 0) as i64,
            Expr::Bin { op: BinOp::And, lhs, rhs } => {
                if self.eval(lhs)? == 0 { 0 } else { (self.eval(rhs)? != 0) as i64 }
            }
            Expr::Bin { op: BinOp::Or, lhs, rhs } => {
                if self.eval(lhs)? != 0 { 1 } else { (self.eval(rhs)? != 0) as i64 }
            }
            Expr::Bin { op, lhs, rhs } => {
                let a = self.eval(lhs)?;
                let b = self.eval(rhs)?;
                match op {
                    BinOp::Add => a.wrapping_add(b),
                    BinOp::Sub => a.wrapping_sub(b),
                    BinOp::Mul => a.wrapping_mul(b),
                    BinOp::Div | BinOp::Rem if b == 0 => return Err("division by zero".to_string()),
                    BinOp::Div => a.wrapping_div(b),
                    BinOp::Rem => a.wrapping_rem(b),
                    BinOp::Eq => (a == b) as i64,
                    BinOp::Ne => (a != b) as i64,
                    BinOp::Lt => (a < b) as i64,
                    BinOp::Le => (a <= b) as i64,
                    BinOp::Gt => (a > b) as i64,
                    BinOp::Ge => (a >= b) as i64,
                    BinOp::And | BinOp::Or => unreachable!(),
                }
            }
            Expr::Call { name, args } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval(a)?);
                }
                self.call(name, &vals)?
            }
        })
    }

    fn call(&mut self, name: &str, args: &[i64]) -> Result<i64, String> {
        self.calls += 1;
        let Some(def) = self.fns.get(name).cloned() else {
            return Err(format!("undefined function {}", name));
        };
        if def.params.len() != args.len() {
            return Err(format!("{} expects {} args, got {}", name, def.params.len(), args.len()));
        }
        let mut frame = HashMap::new();
        for (p, &a) in def.params.iter().zip(args) {
            frame.insert(p.clone(), a);
        }
        self.frames.push(frame);
        let flow = self.exec_block(&def.body);
        self.frames.pop();
        match flow? {
            Flow::Return(v) => Ok(v),
            Flow::Normal => Ok(0),
        }
    }

    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Flow, String> {
        for s in stmts {
            if let Flow::Return(v) = self.exec(s)? {
                return Ok(Flow::Return(v));
            }
        }
        Ok(Flow::Normal)
    }

    fn exec(&mut self, s: &Stmt) -> Result<Flow, String> {
        match s {
            Stmt::Let { name, init } => {
                let v = self.eval(init)?;
                self.frames.last_mut().unwrap().insert(name.clone(), v);
            }
            Stmt::Assign { name, value } => {
                let v = self.eval(value)?;
                let frame = self.frames.last_mut().unwrap();
                match frame.get_mut(name) {
                    Some(slot) => *slot = v,
                    None => return Err(format!("assignment to undeclared {}", name)),
                }
            }
            Stmt::Print { label, value } => {
                let v = self.eval(value)?;
                let line = match label {
                    Some(l) => format!("{}: {}", l, v),
                    None => format!("{}", v),
                };
                println!("{}", line);
                self.output.push(line);
            }
            Stmt::If { cond, then_body, else_body } => {
                let body = if self.eval(cond)? != 0 { then_body } else { else_body };
                return self.exec_block(body);
            }
            Stmt::While { cond, body } => {
                while self.eval(cond)? != 0 {
                    if let Flow::Return(v) = self.exec_block(body)? {
                        return Ok(Flow::Return(v));
                    }
                }
            }
            Stmt::Return(e) => return Ok(Flow::Return(self.eval(e)?)),
            Stmt::ExprStmt(e) => {
                self.eval(e)?;
            }
        }
        Ok(Flow::Normal)
    }
}

fn run(src: &str) -> Result<(), String> {
    let toks = Lexer::new(src).tokenize()?;
    println!("tokens: {}", toks.len());
    let mut p = Parser { toks, pos: 0 };
    let (fns, stmts) = p.program()?;
    let mut names: Vec<&String> = fns.iter().map(|f| &f.name).collect();
    names.sort();
    println!("functions: {:?}", names);
    for s in &stmts {
        if let Stmt::Print { value, .. } = s {
            println!("  ast: {}", value);
        }
    }
    let mut it = Interp { fns: HashMap::new(), frames: vec![HashMap::new()], calls: 0, output: Vec::new() };
    for f in fns {
        it.fns.insert(f.name.clone(), f);
    }
    it.exec_block(&stmts)?;
    let mut globals: Vec<(&String, &i64)> = it.frames[0].iter().collect();
    globals.sort();
    println!("globals: {:?}", globals);
    println!("calls: {} lines: {}", it.calls, it.output.len());
    Ok(())
}

const PROGRAM: &str = r#"
// recursive and iterative functions
fn fib(n) { if n < 2 { return n; } return fib(n - 1) + fib(n - 2); }
fn fact(n) { let r = 1; while n > 1 { r = r * n; n = n - 1; } return r; }
fn gcd(a, b) { while b != 0 { let t = b; b = a % b; a = t; } return a; }
fn collatz(n) {
    let steps = 0;
    while n != 1 {
        if n % 2 == 0 { n = n / 2; } else { n = 3 * n + 1; }
        steps = steps + 1;
    }
    return steps;
}
fn is_prime(n) {
    if n < 2 { return 0; }
    let d = 2;
    while d * d <= n { if n % d == 0 { return 0; } d = d + 1; }
    return 1;
}
fn ack(m, n) {
    if m == 0 { return n + 1; }
    if n == 0 { return ack(m - 1, 1); }
    return ack(m - 1, ack(m, n - 1));
}
let x = 2 + 3 * 4 - 10 / 3;
print "x", x;
print "prec", 1 + 2 * 3 == 7 && !(4 < 3) || 0;
print "neg", -x * -2 - -1;
print "fib20", fib(20);
print "fact20", fact(20);
print "fact25 wraps", fact(25);
print "gcd", gcd(1071, 462);
print "collatz27", collatz(27);
let i = 0;
let count = 0;
let sum = 0;
while i < 100 {
    if is_prime(i) { count = count + 1; sum = sum + i; }
    i = i + 1;
}
print "primes<100", count;
print "sum", sum;
print "ack", ack(2, 3);
if x > 100 { print 1; } else if x > 10 { print 2; } else { print 3; }
print (7 - 10) / 2;
print (7 - 10) % 2;
"#;

fn main() {
    if let Err(e) = run(PROGRAM) {
        println!("error: {}", e);
    }
    // error paths
    for bad in ["let a = 1 +;", "print 5 / (2 - 2);", "let q = $;", "foo(1);", "fn f(a) { return a; } print f(1, 2);", "y = 3;"] {
        println!("--- {}", bad);
        match run(bad) {
            Ok(()) => println!("ok"),
            Err(e) => println!("error: {}", e),
        }
    }
}
