#![allow(dead_code)]
// t12: an SSA-ish IR modelled on ccc's ir/instruction.rs: Vec-indexed arenas,
// big enums with struct variants, a HashMap<String, usize> symbol table,
// optimisation passes mutating through &mut (constant folding, copy
// propagation, branch folding, unreachable-block removal, DCE) and a textual
// emitter built with write! into a String.
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Value(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct BlockId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IrType {
    I32,
    I64,
    Bool,
}

impl IrType {
    fn name(self) -> &'static str {
        match self {
            IrType::I32 => "i32",
            IrType::I64 => "i64",
            IrType::Bool => "i1",
        }
    }
    fn truncate(self, v: i64) -> i64 {
        match self {
            IrType::I32 => v as i32 as i64,
            IrType::I64 => v,
            IrType::Bool => v & 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operand {
    Value(Value),
    Const(i64),
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Value(v) => write!(f, "%{}", v.0),
            Operand::Const(c) => write!(f, "{}", c),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IrBinOp {
    Add,
    Sub,
    Mul,
    SDiv,
    SRem,
    And,
    Or,
    Xor,
    Shl,
    AShr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IrCmpOp {
    Eq,
    Ne,
    Slt,
    Sle,
    Sgt,
    Sge,
}

#[derive(Debug, Clone)]
enum Instruction {
    BinOp { dest: Value, op: IrBinOp, lhs: Operand, rhs: Operand, ty: IrType },
    Cmp { dest: Value, op: IrCmpOp, lhs: Operand, rhs: Operand, ty: IrType },
    Copy { dest: Value, src: Operand },
    Load { dest: Value, global: usize },
    Store { val: Operand, global: usize },
    Call { dest: Option<Value>, func: String, args: Vec<Operand> },
    Phi { dest: Value, ty: IrType, incoming: Vec<(BlockId, Operand)> },
}

impl Instruction {
    fn dest(&self) -> Option<Value> {
        match self {
            Instruction::BinOp { dest, .. }
            | Instruction::Cmp { dest, .. }
            | Instruction::Copy { dest, .. }
            | Instruction::Load { dest, .. }
            | Instruction::Phi { dest, .. } => Some(*dest),
            Instruction::Call { dest, .. } => *dest,
            Instruction::Store { .. } => None,
        }
    }

    fn has_side_effects(&self) -> bool {
        matches!(self, Instruction::Store { .. } | Instruction::Call { .. })
    }

    fn for_each_operand_mut(&mut self, mut f: impl FnMut(&mut Operand)) {
        match self {
            Instruction::BinOp { lhs, rhs, .. } | Instruction::Cmp { lhs, rhs, .. } => {
                f(lhs);
                f(rhs);
            }
            Instruction::Copy { src, .. } => f(src),
            Instruction::Store { val, .. } => f(val),
            Instruction::Call { args, .. } => args.iter_mut().for_each(f),
            Instruction::Phi { incoming, .. } => {
                for (_, op) in incoming.iter_mut() {
                    f(op);
                }
            }
            Instruction::Load { .. } => {}
        }
    }

    fn for_each_used_value(&self, f: &mut dyn FnMut(Value)) {
        let mut visit = |op: &Operand| {
            if let Operand::Value(v) = op {
                f(*v);
            }
        };
        match self {
            Instruction::BinOp { lhs, rhs, .. } | Instruction::Cmp { lhs, rhs, .. } => {
                visit(lhs);
                visit(rhs);
            }
            Instruction::Copy { src, .. } => visit(src),
            Instruction::Store { val, .. } => visit(val),
            Instruction::Call { args, .. } => args.iter().for_each(visit),
            Instruction::Phi { incoming, .. } => incoming.iter().for_each(|(_, o)| visit(o)),
            Instruction::Load { .. } => {}
        }
    }
}

#[derive(Debug, Clone)]
enum Terminator {
    Br(BlockId),
    CondBr { cond: Operand, then_bb: BlockId, else_bb: BlockId },
    Ret(Option<Operand>),
    Unreachable,
}

impl Terminator {
    fn successors(&self) -> Vec<BlockId> {
        match self {
            Terminator::Br(b) => vec![*b],
            Terminator::CondBr { then_bb, else_bb, .. } => vec![*then_bb, *else_bb],
            Terminator::Ret(_) | Terminator::Unreachable => vec![],
        }
    }
}

#[derive(Debug, Clone)]
struct BasicBlock {
    label: BlockId,
    insts: Vec<Instruction>,
    term: Terminator,
}

struct IrFunction {
    name: String,
    params: Vec<Value>,
    blocks: Vec<BasicBlock>,
    next_value: u32,
}

struct Global {
    name: String,
    init: i64,
}

struct IrModule {
    functions: Vec<IrFunction>,
    globals: Vec<Global>,
    func_index: HashMap<String, usize>,
    global_index: HashMap<String, usize>,
}

impl IrModule {
    fn new() -> Self {
        IrModule { functions: Vec::new(), globals: Vec::new(), func_index: HashMap::new(), global_index: HashMap::new() }
    }

    fn add_global(&mut self, name: &str, init: i64) -> usize {
        if let Some(&i) = self.global_index.get(name) {
            return i;
        }
        let idx = self.globals.len();
        self.globals.push(Global { name: name.to_string(), init });
        self.global_index.insert(name.to_string(), idx);
        idx
    }

    fn add_function(&mut self, f: IrFunction) -> usize {
        let idx = self.functions.len();
        self.func_index.insert(f.name.clone(), idx);
        self.functions.push(f);
        idx
    }
}

// --- builder -----------------------------------------------------------------

struct Builder {
    func: IrFunction,
    cur: usize,
}

impl Builder {
    fn new(name: &str, nparams: u32) -> Self {
        let params = (0..nparams).map(Value).collect();
        let entry = BasicBlock { label: BlockId(0), insts: Vec::new(), term: Terminator::Unreachable };
        Builder { func: IrFunction { name: name.to_string(), params, blocks: vec![entry], next_value: nparams }, cur: 0 }
    }
    fn param(&self, i: usize) -> Operand {
        Operand::Value(self.func.params[i])
    }
    fn fresh(&mut self) -> Value {
        let v = Value(self.func.next_value);
        self.func.next_value += 1;
        v
    }
    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.func.blocks.len() as u32);
        self.func.blocks.push(BasicBlock { label: id, insts: Vec::new(), term: Terminator::Unreachable });
        id
    }
    fn switch_to(&mut self, b: BlockId) {
        self.cur = b.0 as usize;
    }
    fn push(&mut self, i: Instruction) {
        self.func.blocks[self.cur].insts.push(i);
    }
    fn bin(&mut self, op: IrBinOp, lhs: Operand, rhs: Operand) -> Operand {
        let dest = self.fresh();
        self.push(Instruction::BinOp { dest, op, lhs, rhs, ty: IrType::I32 });
        Operand::Value(dest)
    }
    fn cmp(&mut self, op: IrCmpOp, lhs: Operand, rhs: Operand) -> Operand {
        let dest = self.fresh();
        self.push(Instruction::Cmp { dest, op, lhs, rhs, ty: IrType::I32 });
        Operand::Value(dest)
    }
    fn copy(&mut self, src: Operand) -> Operand {
        let dest = self.fresh();
        self.push(Instruction::Copy { dest, src });
        Operand::Value(dest)
    }
    fn load(&mut self, global: usize) -> Operand {
        let dest = self.fresh();
        self.push(Instruction::Load { dest, global });
        Operand::Value(dest)
    }
    fn call(&mut self, func: &str, args: Vec<Operand>) -> Operand {
        let dest = self.fresh();
        self.push(Instruction::Call { dest: Some(dest), func: func.to_string(), args });
        Operand::Value(dest)
    }
    fn phi(&mut self, incoming: Vec<(BlockId, Operand)>) -> Operand {
        let dest = self.fresh();
        self.push(Instruction::Phi { dest, ty: IrType::I32, incoming });
        Operand::Value(dest)
    }
    fn term(&mut self, t: Terminator) {
        self.func.blocks[self.cur].term = t;
    }
    fn finish(self) -> IrFunction {
        self.func
    }
}

// --- constant folding (cf. ccc common/const_arith.rs) ------------------------

fn fold_binop(op: IrBinOp, l: i64, r: i64, ty: IrType) -> Option<i64> {
    let v = match op {
        IrBinOp::Add => l.wrapping_add(r),
        IrBinOp::Sub => l.wrapping_sub(r),
        IrBinOp::Mul => l.wrapping_mul(r),
        IrBinOp::SDiv => {
            if r == 0 {
                return None;
            }
            l.wrapping_div(r)
        }
        IrBinOp::SRem => {
            if r == 0 {
                return None;
            }
            l.wrapping_rem(r)
        }
        IrBinOp::And => l & r,
        IrBinOp::Or => l | r,
        IrBinOp::Xor => l ^ r,
        IrBinOp::Shl => l.wrapping_shl(r as u32),
        IrBinOp::AShr => l.wrapping_shr(r as u32),
    };
    Some(ty.truncate(v))
}

fn fold_cmp(op: IrCmpOp, l: i64, r: i64) -> i64 {
    let b = match op {
        IrCmpOp::Eq => l == r,
        IrCmpOp::Ne => l != r,
        IrCmpOp::Slt => l < r,
        IrCmpOp::Sle => l <= r,
        IrCmpOp::Sgt => l > r,
        IrCmpOp::Sge => l >= r,
    };
    b as i64
}

/// Replace foldable instructions by Copy of a constant. Also applies algebraic
/// identities (x+0, x*1, x*0, x-x, x^x).
fn constant_fold(func: &mut IrFunction) -> usize {
    let mut changed = 0;
    for block in func.blocks.iter_mut() {
        for inst in block.insts.iter_mut() {
            let replacement = match *inst {
                Instruction::BinOp { dest, op, lhs: Operand::Const(l), rhs: Operand::Const(r), ty } => {
                    fold_binop(op, l, r, ty).map(|c| Instruction::Copy { dest, src: Operand::Const(c) })
                }
                Instruction::BinOp { dest, op, lhs, rhs, .. } => match (op, lhs, rhs) {
                    (IrBinOp::Add | IrBinOp::Sub | IrBinOp::Or | IrBinOp::Xor | IrBinOp::Shl | IrBinOp::AShr, x, Operand::Const(0))
                    | (IrBinOp::Add | IrBinOp::Or, Operand::Const(0), x)
                    | (IrBinOp::Mul | IrBinOp::SDiv, x, Operand::Const(1))
                    | (IrBinOp::Mul, Operand::Const(1), x) => Some(Instruction::Copy { dest, src: x }),
                    (IrBinOp::Mul | IrBinOp::And, _, Operand::Const(0)) | (IrBinOp::Mul | IrBinOp::And, Operand::Const(0), _) => {
                        Some(Instruction::Copy { dest, src: Operand::Const(0) })
                    }
                    (IrBinOp::Sub | IrBinOp::Xor, Operand::Value(a), Operand::Value(b)) if a == b => {
                        Some(Instruction::Copy { dest, src: Operand::Const(0) })
                    }
                    _ => None,
                },
                Instruction::Cmp { dest, op, lhs: Operand::Const(l), rhs: Operand::Const(r), .. } => {
                    Some(Instruction::Copy { dest, src: Operand::Const(fold_cmp(op, l, r)) })
                }
                Instruction::Phi { dest, ref incoming, .. } => {
                    // phi whose incoming values are all the same operand
                    let first = incoming[0].1;
                    if incoming.iter().all(|(_, o)| *o == first) && first != Operand::Value(dest) {
                        Some(Instruction::Copy { dest, src: first })
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(r) = replacement {
                *inst = r;
                changed += 1;
            }
        }
    }
    changed
}

/// Forward Copy sources into all uses.
fn copy_propagate(func: &mut IrFunction) -> usize {
    let mut map: HashMap<Value, Operand> = HashMap::new();
    for block in &func.blocks {
        for inst in &block.insts {
            if let Instruction::Copy { dest, src } = inst {
                map.insert(*dest, *src);
            }
        }
    }
    // resolve chains
    let resolve = |mut op: Operand, map: &HashMap<Value, Operand>| {
        let mut guard = 0;
        while let Operand::Value(v) = op {
            match map.get(&v) {
                Some(&next) if guard < 64 => {
                    op = next;
                    guard += 1;
                }
                _ => break,
            }
        }
        op
    };
    let mut changed = 0;
    for block in func.blocks.iter_mut() {
        for inst in block.insts.iter_mut() {
            if matches!(inst, Instruction::Copy { .. }) {
                continue;
            }
            inst.for_each_operand_mut(|op| {
                let n = resolve(*op, &map);
                if n != *op {
                    *op = n;
                    changed += 1;
                }
            });
        }
        let t = &mut block.term;
        match t {
            Terminator::CondBr { cond, .. } => {
                let n = resolve(*cond, &map);
                if n != *cond {
                    *cond = n;
                    changed += 1;
                }
            }
            Terminator::Ret(Some(op)) => {
                let n = resolve(*op, &map);
                if n != *op {
                    *op = n;
                    changed += 1;
                }
            }
            _ => {}
        }
    }
    changed
}

fn fold_branches(func: &mut IrFunction) -> usize {
    let mut changed = 0;
    for block in func.blocks.iter_mut() {
        let Terminator::CondBr { cond: Operand::Const(c), then_bb, else_bb } = block.term else {
            continue;
        };
        block.term = Terminator::Br(if c != 0 { then_bb } else { else_bb });
        changed += 1;
    }
    changed
}

fn remove_unreachable(func: &mut IrFunction) -> usize {
    let mut reachable = HashSet::new();
    let mut work = vec![BlockId(0)];
    while let Some(b) = work.pop() {
        if !reachable.insert(b) {
            continue;
        }
        let blk = func.blocks.iter().find(|x| x.label == b).unwrap();
        work.extend(blk.term.successors());
    }
    let before = func.blocks.len();
    func.blocks.retain(|b| reachable.contains(&b.label));
    // prune phi edges from removed blocks
    for block in func.blocks.iter_mut() {
        for inst in block.insts.iter_mut() {
            if let Instruction::Phi { incoming, .. } = inst {
                incoming.retain(|(b, _)| reachable.contains(b));
            }
        }
    }
    before - func.blocks.len()
}

fn dead_code_elim(func: &mut IrFunction) -> usize {
    let mut removed = 0;
    loop {
        let mut used: HashSet<Value> = HashSet::new();
        for block in &func.blocks {
            for inst in &block.insts {
                inst.for_each_used_value(&mut |v| {
                    used.insert(v);
                });
            }
            match &block.term {
                Terminator::CondBr { cond: Operand::Value(v), .. } | Terminator::Ret(Some(Operand::Value(v))) => {
                    used.insert(*v);
                }
                _ => {}
            }
        }
        let mut this_round = 0;
        for block in func.blocks.iter_mut() {
            let n = block.insts.len();
            block.insts.retain(|i| i.has_side_effects() || i.dest().map_or(true, |d| used.contains(&d)));
            this_round += n - block.insts.len();
        }
        if this_round == 0 {
            break;
        }
        removed += this_round;
    }
    removed
}

/// Merge a block into its unique predecessor when that predecessor ends in an
/// unconditional branch to it.
fn merge_blocks(func: &mut IrFunction) -> usize {
    let mut merged = 0;
    loop {
        let mut preds: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        for b in &func.blocks {
            for s in b.term.successors() {
                preds.entry(s).or_default().push(b.label);
            }
        }
        let mut candidate = None;
        for (i, b) in func.blocks.iter().enumerate() {
            if let Terminator::Br(succ) = b.term {
                if succ != b.label && succ != BlockId(0) && preds.get(&succ).map_or(0, |p| p.len()) == 1 {
                    candidate = Some((i, succ));
                    break;
                }
            }
        }
        let Some((pi, succ)) = candidate else { break };
        let si = func.blocks.iter().position(|b| b.label == succ).unwrap();
        let succ_block = func.blocks.remove(si);
        let pi = if si < pi { pi - 1 } else { pi };
        let pred_label = func.blocks[pi].label;
        func.blocks[pi].insts.extend(succ_block.insts);
        func.blocks[pi].term = succ_block.term;
        // successors' phis now come from pred
        for b in func.blocks.iter_mut() {
            for inst in b.insts.iter_mut() {
                if let Instruction::Phi { incoming, .. } = inst {
                    for (from, _) in incoming.iter_mut() {
                        if *from == succ {
                            *from = pred_label;
                        }
                    }
                }
            }
        }
        merged += 1;
    }
    merged
}

// --- emitter -----------------------------------------------------------------

fn binop_name(op: IrBinOp) -> &'static str {
    match op {
        IrBinOp::Add => "add",
        IrBinOp::Sub => "sub",
        IrBinOp::Mul => "mul",
        IrBinOp::SDiv => "sdiv",
        IrBinOp::SRem => "srem",
        IrBinOp::And => "and",
        IrBinOp::Or => "or",
        IrBinOp::Xor => "xor",
        IrBinOp::Shl => "shl",
        IrBinOp::AShr => "ashr",
    }
}

fn emit_function(out: &mut String, module: &IrModule, func: &IrFunction) -> fmt::Result {
    write!(out, "define @{}(", func.name)?;
    for (i, p) in func.params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write!(out, "i32 %{}", p.0)?;
    }
    writeln!(out, ") {{")?;
    for b in &func.blocks {
        writeln!(out, ".L{}:", b.label.0)?;
        for inst in &b.insts {
            out.push_str("  ");
            match inst {
                Instruction::BinOp { dest, op, lhs, rhs, ty } => writeln!(out, "%{} = {} {} {}, {}", dest.0, binop_name(*op), ty.name(), lhs, rhs)?,
                Instruction::Cmp { dest, op, lhs, rhs, ty } => {
                    writeln!(out, "%{} = cmp {} {} {}, {}", dest.0, format!("{:?}", op).to_lowercase(), ty.name(), lhs, rhs)?
                }
                Instruction::Copy { dest, src } => writeln!(out, "%{} = copy {}", dest.0, src)?,
                Instruction::Load { dest, global } => writeln!(out, "%{} = load @{}", dest.0, module.globals[*global].name)?,
                Instruction::Store { val, global } => writeln!(out, "store {}, @{}", val, module.globals[*global].name)?,
                Instruction::Call { dest, func, args } => {
                    if let Some(d) = dest {
                        write!(out, "%{} = ", d.0)?;
                    }
                    let a: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                    let known = if module.func_index.contains_key(func) { "" } else { " ; extern" };
                    writeln!(out, "call @{}({}){}", func, a.join(", "), known)?
                }
                Instruction::Phi { dest, ty, incoming } => {
                    write!(out, "%{} = phi {}", dest.0, ty.name())?;
                    for (i, (b, o)) in incoming.iter().enumerate() {
                        write!(out, "{} [{}, .L{}]", if i == 0 { "" } else { "," }, o, b.0)?;
                    }
                    out.push('\n');
                }
            }
        }
        match &b.term {
            Terminator::Br(t) => writeln!(out, "  br .L{}", t.0)?,
            Terminator::CondBr { cond, then_bb, else_bb } => writeln!(out, "  condbr {}, .L{}, .L{}", cond, then_bb.0, else_bb.0)?,
            Terminator::Ret(Some(v)) => writeln!(out, "  ret {}", v)?,
            Terminator::Ret(None) => writeln!(out, "  ret void")?,
            Terminator::Unreachable => writeln!(out, "  unreachable")?,
        }
    }
    writeln!(out, "}}")
}

fn emit_module(module: &IrModule) -> String {
    let mut out = String::new();
    for g in &module.globals {
        writeln!(out, "@{} = global i32 {}", g.name, g.init).unwrap();
    }
    for f in &module.functions {
        emit_function(&mut out, module, f).unwrap();
    }
    out
}

// --- a tiny interpreter so we can check that passes preserve semantics --------

fn interpret(module: &IrModule, fname: &str, args: &[i64], globals: &mut Vec<i64>, depth: u32) -> i64 {
    let func = &module.functions[module.func_index[fname]];
    let mut vals: HashMap<Value, i64> = HashMap::new();
    for (p, a) in func.params.iter().zip(args) {
        vals.insert(*p, *a);
    }
    let get = |op: &Operand, vals: &HashMap<Value, i64>| match op {
        Operand::Const(c) => *c,
        Operand::Value(v) => vals[v],
    };
    let mut prev = BlockId(u32::MAX);
    let mut cur = BlockId(0);
    loop {
        let block = func.blocks.iter().find(|b| b.label == cur).unwrap();
        for inst in &block.insts {
            match inst {
                Instruction::BinOp { dest, op, lhs, rhs, ty } => {
                    let r = fold_binop(*op, get(lhs, &vals), get(rhs, &vals), *ty).unwrap_or(0);
                    vals.insert(*dest, r);
                }
                Instruction::Cmp { dest, op, lhs, rhs, .. } => {
                    let r = fold_cmp(*op, get(lhs, &vals), get(rhs, &vals));
                    vals.insert(*dest, r);
                }
                Instruction::Copy { dest, src } => {
                    let r = get(src, &vals);
                    vals.insert(*dest, r);
                }
                Instruction::Load { dest, global } => {
                    vals.insert(*dest, globals[*global]);
                }
                Instruction::Store { val, global } => globals[*global] = get(val, &vals),
                Instruction::Call { dest, func, args } => {
                    let a: Vec<i64> = args.iter().map(|o| get(o, &vals)).collect();
                    let r = if module.func_index.contains_key(func) && depth < 50 {
                        interpret(module, func, &a, globals, depth + 1)
                    } else {
                        a.iter().sum::<i64>() * 1000
                    };
                    if let Some(d) = dest {
                        vals.insert(*d, r);
                    }
                }
                Instruction::Phi { dest, incoming, .. } => {
                    let (_, op) = incoming.iter().find(|(b, _)| *b == prev).expect("phi edge");
                    let r = get(op, &vals);
                    vals.insert(*dest, r);
                }
            }
        }
        prev = cur;
        match &block.term {
            Terminator::Br(t) => cur = *t,
            Terminator::CondBr { cond, then_bb, else_bb } => cur = if get(cond, &vals) != 0 { *then_bb } else { *else_bb },
            Terminator::Ret(v) => return v.as_ref().map_or(0, |o| get(o, &vals)),
            Terminator::Unreachable => panic!("unreachable"),
        }
    }
}

fn build_module() -> IrModule {
    let mut m = IrModule::new();
    let counter = m.add_global("counter", 7);
    let limit = m.add_global("limit", 100);
    assert_eq!(m.add_global("counter", 0), counter);

    // fn compute(a, b): lots of foldable junk
    let mut b = Builder::new("compute", 2);
    let c1 = b.bin(IrBinOp::Mul, Operand::Const(6), Operand::Const(7));
    let c2 = b.bin(IrBinOp::Add, c1, Operand::Const(0));
    let x = b.copy(b.param(0));
    let y = b.bin(IrBinOp::Add, x, c2);
    let dead = b.bin(IrBinOp::Mul, y, Operand::Const(99));
    let _dead2 = b.bin(IrBinOp::Add, dead, Operand::Const(1));
    let z = b.bin(IrBinOp::Sub, b.param(1), b.param(1));
    let w = b.bin(IrBinOp::Add, y, z);
    let k = b.cmp(IrCmpOp::Slt, Operand::Const(3), Operand::Const(4));
    let then_bb = b.new_block();
    let else_bb = b.new_block();
    let join = b.new_block();
    b.term(Terminator::CondBr { cond: k, then_bb, else_bb });
    b.switch_to(then_bb);
    let t1 = b.bin(IrBinOp::Shl, w, Operand::Const(1));
    b.term(Terminator::Br(join));
    b.switch_to(else_bb);
    let e1 = b.call("expensive", vec![w]);
    b.term(Terminator::Br(join));
    b.switch_to(join);
    let p = b.phi(vec![(then_bb, t1), (else_bb, e1)]);
    let g = b.load(counter);
    let s = b.bin(IrBinOp::Add, p, g);
    b.push(Instruction::Store { val: s, global: counter });
    let big = b.bin(IrBinOp::Mul, Operand::Const(0x10000), Operand::Const(0x10000));
    let r = b.bin(IrBinOp::Add, s, big);
    b.term(Terminator::Ret(Some(r)));
    m.add_function(b.finish());

    // fn sum_to(n): loop with phis
    let mut b = Builder::new("sum_to", 1);
    let header = b.new_block();
    let body = b.new_block();
    let exit = b.new_block();
    b.term(Terminator::Br(header));
    b.switch_to(header);
    // %i and %acc phis are patched after the body is built
    let i_phi = b.fresh();
    let acc_phi = b.fresh();
    b.push(Instruction::Phi { dest: i_phi, ty: IrType::I32, incoming: vec![] });
    b.push(Instruction::Phi { dest: acc_phi, ty: IrType::I32, incoming: vec![] });
    let lim = b.load(limit);
    let cap = b.copy(lim);
    let c = b.cmp(IrCmpOp::Sle, Operand::Value(i_phi), b.param(0));
    let c2 = b.cmp(IrCmpOp::Slt, Operand::Value(i_phi), cap);
    let both = b.bin(IrBinOp::And, c, c2);
    b.term(Terminator::CondBr { cond: both, then_bb: body, else_bb: exit });
    b.switch_to(body);
    let one = b.copy(Operand::Const(1));
    let sq = b.bin(IrBinOp::Mul, Operand::Value(i_phi), Operand::Value(i_phi));
    let acc2 = b.bin(IrBinOp::Add, Operand::Value(acc_phi), sq);
    let i2 = b.bin(IrBinOp::Add, Operand::Value(i_phi), one);
    b.term(Terminator::Br(header));
    b.switch_to(exit);
    let res = b.bin(IrBinOp::Mul, Operand::Value(acc_phi), Operand::Const(1));
    b.term(Terminator::Ret(Some(res)));
    let mut f = b.finish();
    for inst in f.blocks[header.0 as usize].insts.iter_mut() {
        if let Instruction::Phi { dest, incoming, .. } = inst {
            if *dest == i_phi {
                *incoming = vec![(BlockId(0), Operand::Const(1)), (body, i2)];
            } else {
                *incoming = vec![(BlockId(0), Operand::Const(0)), (body, acc2)];
            }
        }
    }
    m.add_function(f);

    // fn main(): calls the others; a dead branch on a constant false
    let mut b = Builder::new("main", 0);
    let a = b.call("compute", vec![Operand::Const(5), Operand::Const(9)]);
    let s = b.call("sum_to", vec![Operand::Const(10)]);
    let flag = b.cmp(IrCmpOp::Eq, Operand::Const(1), Operand::Const(2));
    let dead_bb = b.new_block();
    let live_bb = b.new_block();
    b.term(Terminator::CondBr { cond: flag, then_bb: dead_bb, else_bb: live_bb });
    b.switch_to(dead_bb);
    b.call("abort", vec![]);
    b.term(Terminator::Unreachable);
    b.switch_to(live_bb);
    let t = b.bin(IrBinOp::Xor, a, s);
    let q = b.bin(IrBinOp::SDiv, t, Operand::Const(-3));
    let rr = b.bin(IrBinOp::SRem, t, Operand::Const(-3));
    let mix = b.bin(IrBinOp::AShr, q, Operand::Const(2));
    let fin = b.bin(IrBinOp::Or, mix, rr);
    b.term(Terminator::Ret(Some(fin)));
    m.add_function(b.finish());
    m
}

fn count_insts(f: &IrFunction) -> usize {
    f.blocks.iter().map(|b| b.insts.len()).sum()
}

fn run_all(m: &IrModule) -> (i64, Vec<i64>, i64) {
    let mut globals: Vec<i64> = m.globals.iter().map(|g| g.init).collect();
    let r = interpret(m, "main", &[], &mut globals, 0);
    let mut g2 = globals.clone();
    let r2 = interpret(m, "sum_to", &[200], &mut g2, 0);
    (r, globals, r2)
}

fn main() {
    let mut m = build_module();
    let mut names: Vec<(&String, &usize)> = m.func_index.iter().collect();
    names.sort();
    println!("symbols: {:?}", names);
    let mut gnames: Vec<(&String, &usize)> = m.global_index.iter().collect();
    gnames.sort();
    println!("globals: {:?}", gnames);
    print!("{}", emit_module(&m));
    let before = run_all(&m);
    println!("before: {:?}", before);

    type Pass = fn(&mut IrFunction) -> usize;
    let passes: [(&str, Pass); 6] = [
        ("constfold", constant_fold),
        ("copyprop", copy_propagate),
        ("branchfold", fold_branches),
        ("unreachable", remove_unreachable),
        ("merge", merge_blocks),
        ("dce", dead_code_elim),
    ];
    for iter in 1..=10 {
        let mut total = 0;
        for f in m.functions.iter_mut() {
            for (pname, pass) in passes.iter() {
                let n = pass(f);
                if n > 0 {
                    println!("iter {} {:>8} {:<12} changed {} (insts now {}, blocks {})", iter, f.name, pname, n, count_insts(f), f.blocks.len());
                }
                total += n;
            }
        }
        if total == 0 {
            println!("fixpoint after {} iterations", iter);
            break;
        }
    }
    let text = emit_module(&m);
    print!("{}", text);
    println!("emitted {} lines, {} bytes", text.lines().count(), text.len());
    let after = run_all(&m);
    println!("after: {:?}", after);
    println!("semantics preserved: {}", before == after);
}
