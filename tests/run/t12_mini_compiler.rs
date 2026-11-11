#![allow(dead_code)]
// t12: an end-to-end mini C compiler split into modules the way ccc is
// (frontend::lexer, frontend::parser, ir, lowering, passes, interp, backend):
// C subset source -> tokens -> AST (Box-heavy enums with struct variants) ->
// basic-block IR over virtual registers with a HashMap<String, usize> symbol
// table -> optimisation passes (constant folding, copy propagation, dead
// code elimination, jump threading) -> IR interpreter (to get the program's
// output) -> textual assembly emitted with write! into a String.

mod frontend {
    pub mod lexer {
        #[derive(Debug, Clone, PartialEq)]
        pub enum Tok {
            Int(i64),
            Ident(String),
            Kw(&'static str),
            P(&'static str),
            Eof,
        }

        const KEYWORDS: [&str; 7] = ["int", "if", "else", "while", "for", "return", "break"];
        const PUNCTS: [&str; 26] = [
            "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "+=", "-=", "++", "--", "+", "-", "*", "/", "%", "<", ">", "=", "!", "(", ")", "{", "}", ";",
        ];

        pub fn lex(src: &str) -> Result<Vec<(Tok, usize)>, String> {
            let b = src.as_bytes();
            let mut i = 0;
            let mut line = 1;
            let mut out = Vec::new();
            'outer: while i < b.len() {
                let c = b[i];
                if c == b'\n' {
                    line += 1;
                    i += 1;
                    continue;
                }
                if c.is_ascii_whitespace() {
                    i += 1;
                    continue;
                }
                if c == b'/' && b.get(i + 1) == Some(&b'/') {
                    while i < b.len() && b[i] != b'\n' {
                        i += 1;
                    }
                    continue;
                }
                if c.is_ascii_digit() {
                    let s = i;
                    while i < b.len() && b[i].is_ascii_digit() {
                        i += 1;
                    }
                    out.push((Tok::Int(src[s..i].parse().map_err(|e| format!("{:?}", e))?), line));
                    continue;
                }
                if c.is_ascii_alphabetic() || c == b'_' {
                    let s = i;
                    while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                        i += 1;
                    }
                    let w = &src[s..i];
                    match KEYWORDS.iter().find(|k| **k == w) {
                        Some(k) => out.push((Tok::Kw(k), line)),
                        None => out.push((Tok::Ident(w.to_string()), line)),
                    }
                    continue;
                }
                if c == b',' {
                    out.push((Tok::P(","), line));
                    i += 1;
                    continue;
                }
                if c == b'[' || c == b']' {
                    out.push((Tok::P(if c == b'[' { "[" } else { "]" }), line));
                    i += 1;
                    continue;
                }
                for p in PUNCTS.iter() {
                    if src[i..].starts_with(p) {
                        out.push((Tok::P(p), line));
                        i += p.len();
                        continue 'outer;
                    }
                }
                return Err(format!("line {}: bad char {:?}", line, c as char));
            }
            out.push((Tok::Eof, line));
            Ok(out)
        }
    }

    pub mod ast {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum BinOp {
            Add,
            Sub,
            Mul,
            Div,
            Mod,
            Shl,
            Shr,
            Lt,
            Le,
            Gt,
            Ge,
            Eq,
            Ne,
            LogAnd,
            LogOr,
        }

        #[derive(Debug, Clone)]
        pub enum Expr {
            Num(i64),
            Var(String),
            Index { array: String, index: Box<Expr> },
            Unary { op: char, operand: Box<Expr> },
            Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
            Assign { target: Box<Expr>, value: Box<Expr> },
            Call { func: String, args: Vec<Expr> },
        }

        #[derive(Debug, Clone)]
        pub enum Stmt {
            Decl { name: String, size: Option<usize>, init: Option<Expr> },
            Expr(Expr),
            If { cond: Expr, then_s: Box<Stmt>, else_s: Option<Box<Stmt>> },
            While { cond: Expr, body: Box<Stmt> },
            For { init: Option<Box<Stmt>>, cond: Option<Expr>, step: Option<Expr>, body: Box<Stmt> },
            Return(Option<Expr>),
            Break,
            Block(Vec<Stmt>),
        }

        #[derive(Debug, Clone)]
        pub struct Function {
            pub name: String,
            pub params: Vec<String>,
            pub body: Vec<Stmt>,
        }

        pub fn count_nodes(s: &Stmt) -> usize {
            fn e(x: &Expr) -> usize {
                1 + match x {
                    Expr::Num(_) | Expr::Var(_) => 0,
                    Expr::Index { index, .. } => e(index),
                    Expr::Unary { operand, .. } => e(operand),
                    Expr::Binary { lhs, rhs, .. } | Expr::Assign { target: lhs, value: rhs } => e(lhs) + e(rhs),
                    Expr::Call { args, .. } => args.iter().map(e).sum(),
                }
            }
            1 + match s {
                Stmt::Decl { init, .. } => init.as_ref().map_or(0, e),
                Stmt::Expr(x) => e(x),
                Stmt::If { cond, then_s, else_s } => e(cond) + count_nodes(then_s) + else_s.as_ref().map_or(0, |s| count_nodes(s)),
                Stmt::While { cond, body } => e(cond) + count_nodes(body),
                Stmt::For { init, cond, step, body } => {
                    init.as_ref().map_or(0, |s| count_nodes(s)) + cond.as_ref().map_or(0, e) + step.as_ref().map_or(0, e) + count_nodes(body)
                }
                Stmt::Return(x) => x.as_ref().map_or(0, e),
                Stmt::Break => 0,
                Stmt::Block(v) => v.iter().map(count_nodes).sum(),
            }
        }
    }

    pub mod parser {
        use super::ast::*;
        use super::lexer::Tok;

        pub struct Parser {
            toks: Vec<(Tok, usize)>,
            pos: usize,
        }

        type R<T> = Result<T, String>;

        impl Parser {
            pub fn new(toks: Vec<(Tok, usize)>) -> Self {
                Parser { toks, pos: 0 }
            }
            fn peek(&self) -> &Tok {
                &self.toks[self.pos].0
            }
            fn next(&mut self) -> Tok {
                let t = self.toks[self.pos].0.clone();
                if !matches!(t, Tok::Eof) {
                    self.pos += 1;
                }
                t
            }
            fn is_p(&self, p: &str) -> bool {
                matches!(self.peek(), Tok::P(q) if *q == p)
            }
            fn is_kw(&self, k: &str) -> bool {
                matches!(self.peek(), Tok::Kw(q) if *q == k)
            }
            fn expect_p(&mut self, p: &str) -> R<()> {
                if self.is_p(p) {
                    self.next();
                    Ok(())
                } else {
                    Err(format!("line {}: expected '{}' got {:?}", self.toks[self.pos].1, p, self.peek()))
                }
            }
            fn ident(&mut self) -> R<String> {
                match self.next() {
                    Tok::Ident(s) => Ok(s),
                    t => Err(format!("expected identifier, got {:?}", t)),
                }
            }
            pub fn program(&mut self) -> R<Vec<Function>> {
                let mut fs = Vec::new();
                while !matches!(self.peek(), Tok::Eof) {
                    if !self.is_kw("int") {
                        return Err(format!("expected 'int' at top level, got {:?}", self.peek()));
                    }
                    self.next();
                    let name = self.ident()?;
                    self.expect_p("(")?;
                    let mut params = Vec::new();
                    while !self.is_p(")") {
                        if self.is_kw("int") {
                            self.next();
                        }
                        params.push(self.ident()?);
                        if self.is_p(",") {
                            self.next();
                        }
                    }
                    self.expect_p(")")?;
                    let Stmt::Block(body) = self.block()? else { unreachable!() };
                    fs.push(Function { name, params, body });
                }
                Ok(fs)
            }
            fn block(&mut self) -> R<Stmt> {
                self.expect_p("{")?;
                let mut v = Vec::new();
                while !self.is_p("}") {
                    v.push(self.stmt()?);
                }
                self.expect_p("}")?;
                Ok(Stmt::Block(v))
            }
            fn decl(&mut self) -> R<Stmt> {
                self.next(); // int
                let name = self.ident()?;
                let mut size = None;
                if self.is_p("[") {
                    self.next();
                    match self.next() {
                        Tok::Int(n) => size = Some(n as usize),
                        t => return Err(format!("array size must be constant, got {:?}", t)),
                    }
                    self.expect_p("]")?;
                }
                let init = if self.is_p("=") {
                    self.next();
                    Some(self.expr()?)
                } else {
                    None
                };
                self.expect_p(";")?;
                Ok(Stmt::Decl { name, size, init })
            }
            fn stmt(&mut self) -> R<Stmt> {
                if self.is_p("{") {
                    return self.block();
                }
                if self.is_kw("int") {
                    return self.decl();
                }
                if self.is_kw("if") {
                    self.next();
                    self.expect_p("(")?;
                    let cond = self.expr()?;
                    self.expect_p(")")?;
                    let then_s = Box::new(self.stmt()?);
                    let else_s = if self.is_kw("else") {
                        self.next();
                        Some(Box::new(self.stmt()?))
                    } else {
                        None
                    };
                    return Ok(Stmt::If { cond, then_s, else_s });
                }
                if self.is_kw("while") {
                    self.next();
                    self.expect_p("(")?;
                    let cond = self.expr()?;
                    self.expect_p(")")?;
                    return Ok(Stmt::While { cond, body: Box::new(self.stmt()?) });
                }
                if self.is_kw("for") {
                    self.next();
                    self.expect_p("(")?;
                    let init = if self.is_p(";") {
                        self.next();
                        None
                    } else if self.is_kw("int") {
                        Some(Box::new(self.decl()?))
                    } else {
                        let e = self.expr()?;
                        self.expect_p(";")?;
                        Some(Box::new(Stmt::Expr(e)))
                    };
                    let cond = if self.is_p(";") { None } else { Some(self.expr()?) };
                    self.expect_p(";")?;
                    let step = if self.is_p(")") { None } else { Some(self.expr()?) };
                    self.expect_p(")")?;
                    return Ok(Stmt::For { init, cond, step, body: Box::new(self.stmt()?) });
                }
                if self.is_kw("return") {
                    self.next();
                    let v = if self.is_p(";") { None } else { Some(self.expr()?) };
                    self.expect_p(";")?;
                    return Ok(Stmt::Return(v));
                }
                if self.is_kw("break") {
                    self.next();
                    self.expect_p(";")?;
                    return Ok(Stmt::Break);
                }
                let e = self.expr()?;
                self.expect_p(";")?;
                Ok(Stmt::Expr(e))
            }
            pub fn expr(&mut self) -> R<Expr> {
                let lhs = self.binary(0)?;
                for (p, op) in [("=", None), ("+=", Some(BinOp::Add)), ("-=", Some(BinOp::Sub))] {
                    if self.is_p(p) {
                        self.next();
                        let rhs = self.expr()?;
                        let value = match op {
                            None => rhs,
                            Some(op) => Expr::Binary { op, lhs: Box::new(lhs.clone()), rhs: Box::new(rhs) },
                        };
                        return Ok(Expr::Assign { target: Box::new(lhs), value: Box::new(value) });
                    }
                }
                Ok(lhs)
            }
            fn binop(&self) -> Option<(BinOp, u8)> {
                let Tok::P(p) = self.peek() else { return None };
                Some(match *p {
                    "||" => (BinOp::LogOr, 1),
                    "&&" => (BinOp::LogAnd, 2),
                    "==" => (BinOp::Eq, 3),
                    "!=" => (BinOp::Ne, 3),
                    "<" => (BinOp::Lt, 4),
                    "<=" => (BinOp::Le, 4),
                    ">" => (BinOp::Gt, 4),
                    ">=" => (BinOp::Ge, 4),
                    "<<" => (BinOp::Shl, 5),
                    ">>" => (BinOp::Shr, 5),
                    "+" => (BinOp::Add, 6),
                    "-" => (BinOp::Sub, 6),
                    "*" => (BinOp::Mul, 7),
                    "/" => (BinOp::Div, 7),
                    "%" => (BinOp::Mod, 7),
                    _ => return None,
                })
            }
            fn binary(&mut self, min: u8) -> R<Expr> {
                let mut lhs = self.unary()?;
                while let Some((op, prec)) = self.binop() {
                    if prec < min {
                        break;
                    }
                    self.next();
                    let rhs = self.binary(prec + 1)?;
                    lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
                Ok(lhs)
            }
            fn unary(&mut self) -> R<Expr> {
                for (p, c) in [("-", '-'), ("!", '!')] {
                    if self.is_p(p) {
                        self.next();
                        return Ok(Expr::Unary { op: c, operand: Box::new(self.unary()?) });
                    }
                }
                if self.is_p("++") || self.is_p("--") {
                    let inc = self.is_p("++");
                    self.next();
                    let target = self.unary()?;
                    let op = if inc { BinOp::Add } else { BinOp::Sub };
                    return Ok(Expr::Assign { target: Box::new(target.clone()), value: Box::new(Expr::Binary { op, lhs: Box::new(target), rhs: Box::new(Expr::Num(1)) }) });
                }
                let e = self.primary()?;
                // postfix ++/-- (only used for effect: yields the new value)
                if self.is_p("++") || self.is_p("--") {
                    let op = if self.is_p("++") { BinOp::Add } else { BinOp::Sub };
                    self.next();
                    return Ok(Expr::Assign { target: Box::new(e.clone()), value: Box::new(Expr::Binary { op, lhs: Box::new(e), rhs: Box::new(Expr::Num(1)) }) });
                }
                Ok(e)
            }
            fn primary(&mut self) -> R<Expr> {
                match self.next() {
                    Tok::Int(n) => Ok(Expr::Num(n)),
                    Tok::P("(") => {
                        let e = self.expr()?;
                        self.expect_p(")")?;
                        Ok(e)
                    }
                    Tok::Ident(name) => {
                        if self.is_p("(") {
                            self.next();
                            let mut args = Vec::new();
                            while !self.is_p(")") {
                                args.push(self.expr()?);
                                if self.is_p(",") {
                                    self.next();
                                }
                            }
                            self.next();
                            Ok(Expr::Call { func: name, args })
                        } else if self.is_p("[") {
                            self.next();
                            let index = self.expr()?;
                            self.expect_p("]")?;
                            Ok(Expr::Index { array: name, index: Box::new(index) })
                        } else {
                            Ok(Expr::Var(name))
                        }
                    }
                    t => Err(format!("unexpected {:?}", t)),
                }
            }
        }
    }
}

mod ir {
    use std::fmt;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Reg(pub u32);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Operand {
        Reg(Reg),
        Imm(i64),
    }

    impl fmt::Display for Operand {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Operand::Reg(r) => write!(f, "v{}", r.0),
                Operand::Imm(i) => write!(f, "${}", i),
            }
        }
    }

    pub use crate::frontend::ast::BinOp;

    #[derive(Debug, Clone)]
    pub enum Inst {
        Mov { dst: Reg, src: Operand },
        Bin { dst: Reg, op: BinOp, a: Operand, b: Operand },
        Neg { dst: Reg, src: Operand },
        Not { dst: Reg, src: Operand },
        LoadSlot { dst: Reg, slot: usize, index: Operand },
        StoreSlot { slot: usize, index: Operand, val: Operand },
        Call { dst: Reg, func: String, args: Vec<Operand> },
    }

    impl Inst {
        pub fn dst(&self) -> Option<Reg> {
            match self {
                Inst::Mov { dst, .. } | Inst::Bin { dst, .. } | Inst::Neg { dst, .. } | Inst::Not { dst, .. } | Inst::LoadSlot { dst, .. } | Inst::Call { dst, .. } => Some(*dst),
                Inst::StoreSlot { .. } => None,
            }
        }
        pub fn operands_mut(&mut self) -> Vec<&mut Operand> {
            match self {
                Inst::Mov { src, .. } | Inst::Neg { src, .. } | Inst::Not { src, .. } => vec![src],
                Inst::Bin { a, b, .. } => vec![a, b],
                Inst::LoadSlot { index, .. } => vec![index],
                Inst::StoreSlot { index, val, .. } => vec![index, val],
                Inst::Call { args, .. } => args.iter_mut().collect(),
            }
        }
        pub fn operands(&self) -> Vec<Operand> {
            let mut c = self.clone();
            c.operands_mut().into_iter().map(|o| *o).collect()
        }
        pub fn pure(&self) -> bool {
            !matches!(self, Inst::StoreSlot { .. } | Inst::Call { .. })
        }
    }

    #[derive(Debug, Clone)]
    pub enum Term {
        Jmp(usize),
        Br { cond: Operand, t: usize, f: usize },
        Ret(Operand),
        None,
    }

    #[derive(Debug, Clone)]
    pub struct Block {
        pub insts: Vec<Inst>,
        pub term: Term,
    }

    #[derive(Debug)]
    pub struct Slot {
        pub name: String,
        pub size: usize,
    }

    #[derive(Debug)]
    pub struct Func {
        pub name: String,
        pub nparams: usize,
        pub blocks: Vec<Block>,
        pub slots: Vec<Slot>,
        pub nregs: u32,
    }

    impl Func {
        pub fn inst_count(&self) -> usize {
            self.blocks.iter().map(|b| b.insts.len()).sum()
        }
    }
}

mod lowering {
    use crate::frontend::ast::*;
    use crate::ir::{Block, Func, Inst, Operand, Reg, Slot, Term};
    use std::collections::HashMap;

    pub struct Lowerer {
        func: Func,
        cur: usize,
        scopes: Vec<HashMap<String, usize>>,
        break_targets: Vec<usize>,
    }

    impl Lowerer {
        fn reg(&mut self) -> Reg {
            let r = Reg(self.func.nregs);
            self.func.nregs += 1;
            r
        }
        fn new_block(&mut self) -> usize {
            self.func.blocks.push(Block { insts: Vec::new(), term: Term::None });
            self.func.blocks.len() - 1
        }
        fn emit(&mut self, i: Inst) {
            self.func.blocks[self.cur].insts.push(i);
        }
        fn terminate(&mut self, t: Term) {
            if matches!(self.func.blocks[self.cur].term, Term::None) {
                self.func.blocks[self.cur].term = t;
            }
        }
        fn lookup(&self, name: &str) -> Result<usize, String> {
            for s in self.scopes.iter().rev() {
                if let Some(&slot) = s.get(name) {
                    return Ok(slot);
                }
            }
            Err(format!("undeclared variable '{}' in {}", name, self.func.name))
        }
        fn declare(&mut self, name: &str, size: usize) -> usize {
            let id = self.func.slots.len();
            self.func.slots.push(Slot { name: name.to_string(), size });
            self.scopes.last_mut().unwrap().insert(name.to_string(), id);
            id
        }

        fn expr(&mut self, e: &Expr) -> Result<Operand, String> {
            Ok(match e {
                Expr::Num(n) => Operand::Imm(*n),
                Expr::Var(v) => {
                    let slot = self.lookup(v)?;
                    let dst = self.reg();
                    self.emit(Inst::LoadSlot { dst, slot, index: Operand::Imm(0) });
                    Operand::Reg(dst)
                }
                Expr::Index { array, index } => {
                    let slot = self.lookup(array)?;
                    let idx = self.expr(index)?;
                    let dst = self.reg();
                    self.emit(Inst::LoadSlot { dst, slot, index: idx });
                    Operand::Reg(dst)
                }
                Expr::Unary { op, operand } => {
                    let v = self.expr(operand)?;
                    let dst = self.reg();
                    self.emit(if *op == '-' { Inst::Neg { dst, src: v } } else { Inst::Not { dst, src: v } });
                    Operand::Reg(dst)
                }
                Expr::Binary { op: op @ (BinOp::LogAnd | BinOp::LogOr), lhs, rhs } => {
                    // short circuit through a temporary slot
                    let tmp = self.declare(&format!("$sc{}", self.func.slots.len()), 1);
                    let l = self.expr(lhs)?;
                    let rhs_bb = self.new_block();
                    let short_bb = self.new_block();
                    let join = self.new_block();
                    let (t, f) = if *op == BinOp::LogAnd { (rhs_bb, short_bb) } else { (short_bb, rhs_bb) };
                    self.terminate(Term::Br { cond: l, t, f });
                    self.cur = short_bb;
                    self.emit(Inst::StoreSlot { slot: tmp, index: Operand::Imm(0), val: Operand::Imm((*op == BinOp::LogOr) as i64) });
                    self.terminate(Term::Jmp(join));
                    self.cur = rhs_bb;
                    let r = self.expr(rhs)?;
                    let z = self.reg();
                    self.emit(Inst::Bin { dst: z, op: BinOp::Ne, a: r, b: Operand::Imm(0) });
                    self.emit(Inst::StoreSlot { slot: tmp, index: Operand::Imm(0), val: Operand::Reg(z) });
                    self.terminate(Term::Jmp(join));
                    self.cur = join;
                    let dst = self.reg();
                    self.emit(Inst::LoadSlot { dst, slot: tmp, index: Operand::Imm(0) });
                    Operand::Reg(dst)
                }
                Expr::Binary { op, lhs, rhs } => {
                    let a = self.expr(lhs)?;
                    let b = self.expr(rhs)?;
                    let dst = self.reg();
                    self.emit(Inst::Bin { dst, op: *op, a, b });
                    Operand::Reg(dst)
                }
                Expr::Assign { target, value } => {
                    let v = self.expr(value)?;
                    match &**target {
                        Expr::Var(name) => {
                            let slot = self.lookup(name)?;
                            self.emit(Inst::StoreSlot { slot, index: Operand::Imm(0), val: v });
                        }
                        Expr::Index { array, index } => {
                            let slot = self.lookup(array)?;
                            let idx = self.expr(index)?;
                            self.emit(Inst::StoreSlot { slot, index: idx, val: v });
                        }
                        other => return Err(format!("cannot assign to {:?}", other)),
                    }
                    v
                }
                Expr::Call { func, args } => {
                    let mut ops = Vec::new();
                    for a in args {
                        ops.push(self.expr(a)?);
                    }
                    let dst = self.reg();
                    self.emit(Inst::Call { dst, func: func.clone(), args: ops });
                    Operand::Reg(dst)
                }
            })
        }

        fn stmt(&mut self, s: &Stmt) -> Result<(), String> {
            match s {
                Stmt::Decl { name, size, init } => {
                    let slot = self.declare(name, size.unwrap_or(1));
                    if let Some(e) = init {
                        let v = self.expr(e)?;
                        self.emit(Inst::StoreSlot { slot, index: Operand::Imm(0), val: v });
                    }
                }
                Stmt::Expr(e) => {
                    self.expr(e)?;
                }
                Stmt::If { cond, then_s, else_s } => {
                    let c = self.expr(cond)?;
                    let tb = self.new_block();
                    let eb = self.new_block();
                    let join = if else_s.is_some() { self.new_block() } else { eb };
                    self.terminate(Term::Br { cond: c, t: tb, f: eb });
                    self.cur = tb;
                    self.stmt(then_s)?;
                    self.terminate(Term::Jmp(join));
                    if let Some(e) = else_s {
                        self.cur = eb;
                        self.stmt(e)?;
                        self.terminate(Term::Jmp(join));
                    }
                    self.cur = join;
                }
                Stmt::While { cond, body } => {
                    let head = self.new_block();
                    let bodyb = self.new_block();
                    let exit = self.new_block();
                    self.terminate(Term::Jmp(head));
                    self.cur = head;
                    let c = self.expr(cond)?;
                    self.terminate(Term::Br { cond: c, t: bodyb, f: exit });
                    self.cur = bodyb;
                    self.break_targets.push(exit);
                    self.stmt(body)?;
                    self.break_targets.pop();
                    self.terminate(Term::Jmp(head));
                    self.cur = exit;
                }
                Stmt::For { init, cond, step, body } => {
                    self.scopes.push(HashMap::new());
                    if let Some(i) = init {
                        self.stmt(i)?;
                    }
                    let head = self.new_block();
                    let bodyb = self.new_block();
                    let stepb = self.new_block();
                    let exit = self.new_block();
                    self.terminate(Term::Jmp(head));
                    self.cur = head;
                    match cond {
                        Some(c) => {
                            let c = self.expr(c)?;
                            self.terminate(Term::Br { cond: c, t: bodyb, f: exit });
                        }
                        None => self.terminate(Term::Jmp(bodyb)),
                    }
                    self.cur = bodyb;
                    self.break_targets.push(exit);
                    self.stmt(body)?;
                    self.break_targets.pop();
                    self.terminate(Term::Jmp(stepb));
                    self.cur = stepb;
                    if let Some(s) = step {
                        self.expr(s)?;
                    }
                    self.terminate(Term::Jmp(head));
                    self.cur = exit;
                    self.scopes.pop();
                }
                Stmt::Return(v) => {
                    let v = match v {
                        Some(e) => self.expr(e)?,
                        None => Operand::Imm(0),
                    };
                    self.terminate(Term::Ret(v));
                    self.cur = self.new_block();
                }
                Stmt::Break => {
                    let Some(&t) = self.break_targets.last() else {
                        return Err("break outside loop".into());
                    };
                    self.terminate(Term::Jmp(t));
                    self.cur = self.new_block();
                }
                Stmt::Block(v) => {
                    self.scopes.push(HashMap::new());
                    for s in v {
                        self.stmt(s)?;
                    }
                    self.scopes.pop();
                }
            }
            Ok(())
        }
    }

    pub fn lower(f: &Function) -> Result<Func, String> {
        let mut l = Lowerer {
            func: Func { name: f.name.clone(), nparams: f.params.len(), blocks: vec![Block { insts: vec![], term: Term::None }], slots: vec![], nregs: 0 },
            cur: 0,
            scopes: vec![HashMap::new()],
            break_targets: vec![],
        };
        for p in &f.params {
            l.declare(p, 1);
        }
        for s in &f.body {
            l.stmt(s)?;
        }
        l.terminate(Term::Ret(Operand::Imm(0)));
        for b in l.func.blocks.iter_mut() {
            if matches!(b.term, Term::None) {
                b.term = Term::Ret(Operand::Imm(0));
            }
        }
        Ok(l.func)
    }
}

mod passes {
    use crate::ir::*;
    use std::collections::{HashMap, HashSet};

    pub fn eval_bin(op: BinOp, a: i64, b: i64) -> Option<i64> {
        Some(match op {
            BinOp::Add => a.wrapping_add(b),
            BinOp::Sub => a.wrapping_sub(b),
            BinOp::Mul => a.wrapping_mul(b),
            BinOp::Div => {
                if b == 0 {
                    return None;
                }
                a.wrapping_div(b)
            }
            BinOp::Mod => {
                if b == 0 {
                    return None;
                }
                a.wrapping_rem(b)
            }
            BinOp::Shl => a.wrapping_shl(b as u32),
            BinOp::Shr => a.wrapping_shr(b as u32),
            BinOp::Lt => (a < b) as i64,
            BinOp::Le => (a <= b) as i64,
            BinOp::Gt => (a > b) as i64,
            BinOp::Ge => (a >= b) as i64,
            BinOp::Eq => (a == b) as i64,
            BinOp::Ne => (a != b) as i64,
            BinOp::LogAnd => (a != 0 && b != 0) as i64,
            BinOp::LogOr => (a != 0 || b != 0) as i64,
        })
    }

    pub fn const_fold(f: &mut Func) -> usize {
        let mut n = 0;
        for b in f.blocks.iter_mut() {
            for i in b.insts.iter_mut() {
                let new = match *i {
                    Inst::Bin { dst, op, a: Operand::Imm(x), b: Operand::Imm(y) } => eval_bin(op, x, y).map(|v| Inst::Mov { dst, src: Operand::Imm(v) }),
                    Inst::Bin { dst, op: BinOp::Mul, a, b: Operand::Imm(1) } | Inst::Bin { dst, op: BinOp::Add, a, b: Operand::Imm(0) } => Some(Inst::Mov { dst, src: a }),
                    Inst::Neg { dst, src: Operand::Imm(x) } => Some(Inst::Mov { dst, src: Operand::Imm(x.wrapping_neg()) }),
                    Inst::Not { dst, src: Operand::Imm(x) } => Some(Inst::Mov { dst, src: Operand::Imm((x == 0) as i64) }),
                    _ => None,
                };
                if let Some(new) = new {
                    *i = new;
                    n += 1;
                }
            }
            if let Term::Br { cond: Operand::Imm(c), t, f: fl } = b.term {
                b.term = Term::Jmp(if c != 0 { t } else { fl });
                n += 1;
            }
        }
        n
    }

    pub fn copy_prop(f: &mut Func) -> usize {
        let mut map: HashMap<Reg, Operand> = HashMap::new();
        for b in &f.blocks {
            for i in &b.insts {
                if let Inst::Mov { dst, src } = i {
                    map.insert(*dst, *src);
                }
            }
        }
        let mut n = 0;
        let subst = |o: &mut Operand, n: &mut usize| {
            let mut guard = 0;
            while let Operand::Reg(r) = *o {
                match map.get(&r) {
                    Some(s) if guard < 32 => {
                        *o = *s;
                        *n += 1;
                        guard += 1;
                    }
                    _ => break,
                }
            }
        };
        for b in f.blocks.iter_mut() {
            for i in b.insts.iter_mut() {
                for o in i.operands_mut() {
                    subst(o, &mut n);
                }
            }
            match &mut b.term {
                Term::Br { cond, .. } => subst(cond, &mut n),
                Term::Ret(v) => subst(v, &mut n),
                _ => {}
            }
        }
        n
    }

    pub fn dce(f: &mut Func) -> usize {
        let mut removed = 0;
        loop {
            let mut used: HashSet<Reg> = HashSet::new();
            for b in &f.blocks {
                for i in &b.insts {
                    for o in i.operands() {
                        if let Operand::Reg(r) = o {
                            used.insert(r);
                        }
                    }
                }
                match &b.term {
                    Term::Br { cond: Operand::Reg(r), .. } | Term::Ret(Operand::Reg(r)) => {
                        used.insert(*r);
                    }
                    _ => {}
                }
            }
            let before = f.inst_count();
            for b in f.blocks.iter_mut() {
                b.insts.retain(|i| !i.pure() || i.dst().map_or(true, |d| used.contains(&d)));
            }
            let r = before - f.inst_count();
            if r == 0 {
                return removed;
            }
            removed += r;
        }
    }

    /// Jump threading: a jump to an empty block that just jumps on is redirected.
    pub fn thread_jumps(f: &mut Func) -> usize {
        let target = |f: &Func, mut b: usize| {
            let mut hops = 0;
            while f.blocks[b].insts.is_empty() && hops < 16 {
                if let Term::Jmp(n) = f.blocks[b].term {
                    if n == b {
                        break;
                    }
                    b = n;
                    hops += 1;
                } else {
                    break;
                }
            }
            b
        };
        let mut n = 0;
        for i in 0..f.blocks.len() {
            let new_term = match f.blocks[i].term.clone() {
                Term::Jmp(t) => {
                    let nt = target(f, t);
                    if nt != t { Some(Term::Jmp(nt)) } else { None }
                }
                Term::Br { cond, t, f: fl } => {
                    let (nt, nf) = (target(f, t), target(f, fl));
                    if nt != t || nf != fl { Some(Term::Br { cond, t: nt, f: nf }) } else { None }
                }
                _ => None,
            };
            if let Some(t) = new_term {
                f.blocks[i].term = t;
                n += 1;
            }
        }
        n
    }

    pub fn reachable(f: &Func) -> Vec<bool> {
        let mut seen = vec![false; f.blocks.len()];
        let mut stack = vec![0];
        while let Some(b) = stack.pop() {
            if std::mem::replace(&mut seen[b], true) {
                continue;
            }
            match f.blocks[b].term {
                Term::Jmp(t) => stack.push(t),
                Term::Br { t, f: fl, .. } => {
                    stack.push(t);
                    stack.push(fl);
                }
                _ => {}
            }
        }
        seen
    }
}

mod interp {
    use crate::ir::*;
    use crate::passes::eval_bin;
    use std::collections::HashMap;

    pub struct Machine<'a> {
        pub funcs: &'a [Func],
        pub index: HashMap<String, usize>,
        pub output: Vec<i64>,
        pub steps: u64,
    }

    impl<'a> Machine<'a> {
        pub fn call(&mut self, name: &str, args: &[i64]) -> Result<i64, String> {
            if name == "print" {
                self.output.extend_from_slice(args);
                return Ok(0);
            }
            let Some(&fi) = self.index.get(name) else {
                return Err(format!("call to undefined function {}", name));
            };
            let f = &self.funcs[fi];
            let mut mem: Vec<Vec<i64>> = f.slots.iter().map(|s| vec![0; s.size]).collect();
            for (i, a) in args.iter().enumerate().take(f.nparams) {
                mem[i][0] = *a;
            }
            let mut regs = vec![0i64; f.nregs as usize];
            let val = |o: &Operand, regs: &Vec<i64>| match o {
                Operand::Imm(i) => *i,
                Operand::Reg(r) => regs[r.0 as usize],
            };
            let mut b = 0usize;
            loop {
                for inst in &f.blocks[b].insts {
                    self.steps += 1;
                    if self.steps > 5_000_000 {
                        return Err("step limit".into());
                    }
                    match inst {
                        Inst::Mov { dst, src } => regs[dst.0 as usize] = val(src, &regs),
                        Inst::Bin { dst, op, a, b } => {
                            regs[dst.0 as usize] = eval_bin(*op, val(a, &regs), val(b, &regs)).ok_or("division by zero")?;
                        }
                        Inst::Neg { dst, src } => regs[dst.0 as usize] = val(src, &regs).wrapping_neg(),
                        Inst::Not { dst, src } => regs[dst.0 as usize] = (val(src, &regs) == 0) as i64,
                        Inst::LoadSlot { dst, slot, index } => {
                            let i = val(index, &regs);
                            let v = *mem[*slot].get(i as usize).ok_or_else(|| format!("index {} out of bounds for {}", i, f.slots[*slot].name))?;
                            regs[dst.0 as usize] = v;
                        }
                        Inst::StoreSlot { slot, index, val: v } => {
                            let i = val(index, &regs) as usize;
                            let x = val(v, &regs);
                            let len = mem[*slot].len();
                            match mem[*slot].get_mut(i) {
                                Some(cell) => *cell = x,
                                None => return Err(format!("store index {} out of bounds (len {})", i, len)),
                            }
                        }
                        Inst::Call { dst, func, args } => {
                            let a: Vec<i64> = args.iter().map(|o| val(o, &regs)).collect();
                            regs[dst.0 as usize] = self.call(func, &a)?;
                        }
                    }
                }
                self.steps += 1;
                if self.steps > 5_000_000 {
                    return Err("step limit".into());
                }
                match &f.blocks[b].term {
                    Term::Jmp(t) => b = *t,
                    Term::Br { cond, t, f: fl } => b = if val(cond, &regs) != 0 { *t } else { *fl },
                    Term::Ret(v) => return Ok(val(v, &regs)),
                    Term::None => return Err("fell off block".into()),
                }
            }
        }
    }
}

mod backend {
    use crate::ir::*;
    use std::fmt::Write;

    fn opnd(o: &Operand) -> String {
        match o {
            Operand::Imm(i) => format!("${}", i),
            Operand::Reg(r) => format!("{}(%rbp)", -8 * (r.0 as i64 + 1)),
        }
    }

    pub fn emit(f: &Func, live: &[bool]) -> String {
        let mut s = String::new();
        let slot_base: Vec<usize> = f
            .slots
            .iter()
            .scan(0usize, |acc, sl| {
                let b = *acc;
                *acc += sl.size;
                Some(b)
            })
            .collect();
        let frame = 8 * (f.nregs as usize + f.slots.iter().map(|s| s.size).sum::<usize>());
        let frame = (frame + 15) & !15;
        writeln!(s, "{}:", f.name).unwrap();
        writeln!(s, "  push %rbp\n  mov %rsp, %rbp\n  sub ${}, %rsp", frame).unwrap();
        let slot_off = |slot: usize| -8 * (f.nregs as i64 + 1 + slot_base[slot] as i64);
        for (bi, b) in f.blocks.iter().enumerate() {
            if !live[bi] {
                continue;
            }
            writeln!(s, ".L{}_{}:", f.name, bi).unwrap();
            for i in &b.insts {
                match i {
                    Inst::Mov { dst, src } => writeln!(s, "  mov {}, %rax\n  mov %rax, {}", opnd(src), opnd(&Operand::Reg(*dst))),
                    Inst::Bin { dst, op, a, b } => {
                        let m = format!("{:?}", op).to_lowercase();
                        writeln!(s, "  mov {}, %rax\n  {} {}, %rax\n  mov %rax, {}", opnd(a), m, opnd(b), opnd(&Operand::Reg(*dst)))
                    }
                    Inst::Neg { dst, src } => writeln!(s, "  mov {}, %rax\n  neg %rax\n  mov %rax, {}", opnd(src), opnd(&Operand::Reg(*dst))),
                    Inst::Not { dst, src } => writeln!(s, "  cmpq $0, {}\n  sete %al\n  movzx %al, %rax\n  mov %rax, {}", opnd(src), opnd(&Operand::Reg(*dst))),
                    Inst::LoadSlot { dst, slot, index: Operand::Imm(k) } => {
                        writeln!(s, "  mov {}(%rbp), %rax  # {}\n  mov %rax, {}", slot_off(*slot) - 8 * k, f.slots[*slot].name, opnd(&Operand::Reg(*dst)))
                    }
                    Inst::LoadSlot { dst, slot, index } => writeln!(
                        s,
                        "  mov {}, %rcx\n  neg %rcx\n  mov {}(%rbp,%rcx,8), %rax  # {}[]\n  mov %rax, {}",
                        opnd(index),
                        slot_off(*slot),
                        f.slots[*slot].name,
                        opnd(&Operand::Reg(*dst))
                    ),
                    Inst::StoreSlot { slot, index: Operand::Imm(k), val } => {
                        writeln!(s, "  mov {}, %rax\n  mov %rax, {}(%rbp)  # {}", opnd(val), slot_off(*slot) - 8 * k, f.slots[*slot].name)
                    }
                    Inst::StoreSlot { slot, index, val } => {
                        writeln!(s, "  mov {}, %rcx\n  neg %rcx\n  mov {}, %rax\n  mov %rax, {}(%rbp,%rcx,8)  # {}[]", opnd(index), opnd(val), slot_off(*slot), f.slots[*slot].name)
                    }
                    Inst::Call { dst, func, args } => {
                        const REGS: [&str; 6] = ["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"];
                        for (k, a) in args.iter().enumerate() {
                            writeln!(s, "  mov {}, {}", opnd(a), REGS[k]).unwrap();
                        }
                        writeln!(s, "  call {}\n  mov %rax, {}", func, opnd(&Operand::Reg(*dst)))
                    }
                }
                .unwrap();
            }
            match &b.term {
                Term::Jmp(t) => writeln!(s, "  jmp .L{}_{}", f.name, t),
                Term::Br { cond, t, f: fl } => writeln!(s, "  cmpq $0, {}\n  jne .L{}_{}\n  jmp .L{}_{}", opnd(cond), f.name, t, f.name, fl),
                Term::Ret(v) => writeln!(s, "  mov {}, %rax\n  leave\n  ret", opnd(v)),
                Term::None => writeln!(s, "  ud2"),
            }
            .unwrap();
        }
        s
    }
}

use std::collections::HashMap;

const PROGRAM: &str = r#"
// sieve, recursion, loops with break, arrays, short-circuit logic
int fib(int n) { if (n < 2) return n; return fib(n - 1) + fib(n - 2); }
int gcd(int a, int b) { while (b != 0) { int t = b; b = a % b; a = t; } return a; }
int sieve(int limit) {
    int composite[100];
    int count = 0;
    for (int i = 2; i < limit; i++) {
        if (!composite[i]) {
            count += 1;
            print(i);
            for (int j = i * i; j < limit; j += i) composite[j] = 1;
        }
    }
    return count;
}
int first_square_over(int x) {
    int i = 0;
    while (1) { if (i * i > x) break; i++; }
    return i;
}
int logic(int a, int b) {
    int r = 0;
    if (a > 0 && b / a > 2) r = r + 1;
    if (a == 0 || b == 0) r = r + 10;
    if (!(a < b) || 0) r = r + 100;
    return r;
}
int consts() {
    int k = (3 + 4) * 2 - 10 / 3 << 1;
    int dead = 12345 * 678;
    if (1 < 2) return k + -(-5) % 3;
    return 0;
}
int main() {
    print(fib(15));
    print(gcd(1071, 462));
    print(sieve(60));
    print(first_square_over(50));
    print(logic(2, 9));
    print(logic(0, 5));
    print(logic(7, 3));
    print(consts());
    int acc[5];
    for (int i = 0; i < 5; i++) acc[i] = i * i - 3;
    int s = 0;
    for (int i = 4; i >= 0; i--) s = s * 10 + acc[i] % 7;
    print(s);
    return s % 256;
}
"#;

fn compile_and_run(src: &str, show_asm: bool) -> Result<(), String> {
    let toks = frontend::lexer::lex(src)?;
    println!("tokens: {}", toks.len());
    let fns = frontend::parser::Parser::new(toks).program()?;
    for f in &fns {
        let nodes: usize = f.body.iter().map(frontend::ast::count_nodes).sum();
        println!("fn {}({}) ast nodes: {}", f.name, f.params.join(", "), nodes);
    }
    let mut funcs = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for f in &fns {
        let lf = lowering::lower(f)?;
        index.insert(lf.name.clone(), funcs.len());
        funcs.push(lf);
    }
    let mut syms: Vec<(&String, &usize)> = index.iter().collect();
    syms.sort_by_key(|p| *p.1);
    println!("symbols: {:?}", syms);

    // run before optimisation
    let mut m = interp::Machine { funcs: &funcs, index: index.clone(), output: vec![], steps: 0 };
    let r0 = m.call("main", &[])?;
    let (out0, steps0) = (m.output.clone(), m.steps);
    println!("unoptimised: ret={} steps={} output={:?}", r0, steps0, out0);

    for f in funcs.iter_mut() {
        let before = (f.inst_count(), f.blocks.len());
        let mut log = Vec::new();
        for round in 0..8 {
            let a = passes::const_fold(f);
            let b = passes::copy_prop(f);
            let c = passes::dce(f);
            let d = passes::thread_jumps(f);
            if a + b + c + d == 0 {
                break;
            }
            log.push(format!("r{}:{}/{}/{}/{}", round, a, b, c, d));
        }
        let live = passes::reachable(f);
        println!("opt {:<18} insts {:>3} -> {:>3}, blocks {:>2} (live {:>2}), slots {}, regs {} [{}]", f.name, before.0, f.inst_count(), before.1, live.iter().filter(|x| **x).count(), f.slots.len(), f.nregs, log.join(" "));
    }
    let mut m = interp::Machine { funcs: &funcs, index: index.clone(), output: vec![], steps: 0 };
    let r1 = m.call("main", &[])?;
    println!("optimised:   ret={} steps={} output={:?}", r1, m.steps, m.output);
    println!("same result: {} fewer steps: {}", r1 == r0 && m.output == out0, m.steps < steps0);

    let mut asm = String::new();
    for f in &funcs {
        let live = passes::reachable(f);
        asm.push_str(&backend::emit(f, &live));
    }
    let lines = asm.lines().count();
    let checksum = asm.bytes().fold(0u32, |h, b| h.rotate_left(5) ^ b as u32);
    println!("asm: {} lines, checksum {:08x}", lines, checksum);
    if show_asm {
        let consts_start = asm.find("consts:").unwrap();
        let consts_end = asm[consts_start..].find("main:").map(|e| consts_start + e).unwrap_or(asm.len());
        print!("{}", &asm[consts_start..consts_end]);
        for l in asm.lines().filter(|l| l.contains("call")) {
            println!("{}", l.trim());
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = compile_and_run(PROGRAM, true) {
        println!("error: {}", e);
    }
    let bad = [
        "int main() { return x; }",
        "int main() { break; }",
        "int main() { int a[3]; a[5] = 1; return 0; }",
        "int main() { return 1 / (2 - 2); }",
        "int main() { return foo(1); }",
        "int main() { int x = 3 $ 4; }",
        "int main( { }",
        "int main() { while (1) { } }",
    ];
    for src in bad {
        println!("--- {}", src);
        match compile_and_run(src, false) {
            Ok(()) => println!("ok"),
            Err(e) => println!("error: {}", e),
        }
    }
}
