#![allow(dead_code)]
// t12: constant-expression arithmetic with C width/signedness semantics.
// wrap_result, unsigned_op, bool_to_i64 and eval_const_binop_int are lifted
// verbatim from ccc src/common/const_arith.rs (with a local IrConst/BinOp);
// the i128 path and float path are simplified versions of the same file.
// The driver evaluates a grid of operand/type combinations and prints every
// result, plus a small constant-expression tree folder.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IrConst {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    F32(f32),
    F64(f64),
}

impl IrConst {
    fn to_i64(&self) -> Option<i64> {
        Some(match *self {
            IrConst::I8(v) => v as i64,
            IrConst::I16(v) => v as i64,
            IrConst::I32(v) => v as i64,
            IrConst::I64(v) => v,
            IrConst::I128(v) => v as i64,
            IrConst::F32(v) => v as i64,
            IrConst::F64(v) => v as i64,
        })
    }
    fn to_i128(&self) -> Option<i128> {
        Some(match *self {
            IrConst::I128(v) => v,
            IrConst::F32(_) | IrConst::F64(_) => return None,
            _ => self.to_i64()? as i128,
        })
    }
    fn to_f64(&self) -> Option<f64> {
        Some(match *self {
            IrConst::F32(v) => v as f64,
            IrConst::F64(v) => v,
            IrConst::I128(v) => v as f64,
            _ => self.to_i64()? as f64,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    LogicalAnd,
    LogicalOr,
}

// === lifted from ccc src/common/const_arith.rs ===============================

/// Wrap an i64 result to 32-bit width if `is_32bit` is true, otherwise return as-is.
#[inline]
fn wrap_result(v: i64, is_32bit: bool) -> i64 {
    if is_32bit { v as i32 as i64 } else { v }
}

/// Perform an unsigned binary operation, handling 32-bit vs 64-bit width.
#[inline]
fn unsigned_op(l: i64, r: i64, is_32bit: bool, op: fn(u64, u64) -> u64) -> i64 {
    if is_32bit {
        op(l as u32 as u64, r as u32 as u64) as u32 as i64
    } else {
        op(l as u64, r as u64) as i64
    }
}

/// Convert a boolean to i64 (1 for true, 0 for false).
#[inline]
fn bool_to_i64(b: bool) -> i64 {
    if b { 1 } else { 0 }
}

fn eval_const_binop_int(op: &BinOp, l: i64, r: i64, is_32bit: bool, is_unsigned: bool) -> Option<IrConst> {
    let result = match op {
        BinOp::Add => wrap_result(l.wrapping_add(r), is_32bit),
        BinOp::Sub => wrap_result(l.wrapping_sub(r), is_32bit),
        BinOp::Mul => wrap_result(l.wrapping_mul(r), is_32bit),
        BinOp::Div => {
            if r == 0 { return None; }
            if is_unsigned {
                unsigned_op(l, r, is_32bit, u64::wrapping_div)
            } else {
                wrap_result(l.wrapping_div(r), is_32bit)
            }
        }
        BinOp::Mod => {
            if r == 0 { return None; }
            if is_unsigned {
                unsigned_op(l, r, is_32bit, u64::wrapping_rem)
            } else {
                wrap_result(l.wrapping_rem(r), is_32bit)
            }
        }
        BinOp::BitAnd => l & r,
        BinOp::BitOr => l | r,
        BinOp::BitXor => l ^ r,
        BinOp::Shl => wrap_result(l.wrapping_shl(r as u32), is_32bit),
        BinOp::Shr => {
            if is_unsigned {
                unsigned_op(l, r, is_32bit, |a, b| a.wrapping_shr(b as u32))
            } else if is_32bit {
                (l as i32).wrapping_shr(r as u32) as i64
            } else {
                l.wrapping_shr(r as u32)
            }
        }
        BinOp::Eq => {
            if is_32bit { bool_to_i64(l as u32 == r as u32) }
            else if is_unsigned { bool_to_i64(l as u64 == r as u64) }
            else { bool_to_i64(l == r) }
        }
        BinOp::Ne => {
            if is_32bit { bool_to_i64(l as u32 != r as u32) }
            else if is_unsigned { bool_to_i64(l as u64 != r as u64) }
            else { bool_to_i64(l != r) }
        }
        BinOp::Lt => {
            if is_32bit { if is_unsigned { bool_to_i64((l as u32) < (r as u32)) } else { bool_to_i64((l as i32) < (r as i32)) } }
            else if is_unsigned { bool_to_i64((l as u64) < (r as u64)) }
            else { bool_to_i64(l < r) }
        }
        BinOp::Gt => {
            if is_32bit { if is_unsigned { bool_to_i64((l as u32) > (r as u32)) } else { bool_to_i64((l as i32) > (r as i32)) } }
            else if is_unsigned { bool_to_i64((l as u64) > (r as u64)) }
            else { bool_to_i64(l > r) }
        }
        BinOp::Le => {
            if is_32bit { if is_unsigned { bool_to_i64((l as u32) <= (r as u32)) } else { bool_to_i64((l as i32) <= (r as i32)) } }
            else if is_unsigned { bool_to_i64((l as u64) <= (r as u64)) }
            else { bool_to_i64(l <= r) }
        }
        BinOp::Ge => {
            if is_32bit { if is_unsigned { bool_to_i64((l as u32) >= (r as u32)) } else { bool_to_i64((l as i32) >= (r as i32)) } }
            else if is_unsigned { bool_to_i64((l as u64) >= (r as u64)) }
            else { bool_to_i64(l >= r) }
        }
        BinOp::LogicalAnd => bool_to_i64(l != 0 && r != 0),
        BinOp::LogicalOr => bool_to_i64(l != 0 || r != 0),
    };
    if is_32bit {
        if is_unsigned {
            Some(IrConst::I64(result as u32 as i64))
        } else {
            Some(IrConst::I32(result as i32))
        }
    } else {
        Some(IrConst::I64(result))
    }
}

// === simplified from the same file =============================================

fn eval_const_binop_i128(op: &BinOp, lhs: &IrConst, rhs: &IrConst, is_unsigned: bool) -> Option<IrConst> {
    let l = lhs.to_i128()?;
    let r = rhs.to_i128()?;
    let b = |x: bool| Some(IrConst::I64(x as i64));
    let v = match op {
        BinOp::Add => l.wrapping_add(r),
        BinOp::Sub => l.wrapping_sub(r),
        BinOp::Mul => l.wrapping_mul(r),
        BinOp::Div | BinOp::Mod if r == 0 => return None,
        BinOp::Div => if is_unsigned { ((l as u128) / (r as u128)) as i128 } else { l.wrapping_div(r) },
        BinOp::Mod => if is_unsigned { ((l as u128) % (r as u128)) as i128 } else { l.wrapping_rem(r) },
        BinOp::BitAnd => l & r,
        BinOp::BitOr => l | r,
        BinOp::BitXor => l ^ r,
        BinOp::Shl => l.wrapping_shl(r as u32),
        BinOp::Shr => if is_unsigned { ((l as u128).wrapping_shr(r as u32)) as i128 } else { l.wrapping_shr(r as u32) },
        BinOp::Eq => return b(l == r),
        BinOp::Ne => return b(l != r),
        BinOp::Lt => return b(if is_unsigned { (l as u128) < (r as u128) } else { l < r }),
        BinOp::Gt => return b(if is_unsigned { (l as u128) > (r as u128) } else { l > r }),
        BinOp::Le => return b(if is_unsigned { (l as u128) <= (r as u128) } else { l <= r }),
        BinOp::Ge => return b(if is_unsigned { (l as u128) >= (r as u128) } else { l >= r }),
        BinOp::LogicalAnd => return b(l != 0 && r != 0),
        BinOp::LogicalOr => return b(l != 0 || r != 0),
    };
    Some(IrConst::I128(v))
}

pub fn eval_const_binop_float(op: &BinOp, lhs: &IrConst, rhs: &IrConst) -> Option<IrConst> {
    let use_f32 = matches!(lhs, IrConst::F32(_)) && matches!(rhs, IrConst::F32(_));
    let l = lhs.to_f64()?;
    let r = rhs.to_f64()?;
    let make_float = |v: f64| -> IrConst {
        if use_f32 {
            IrConst::F32(v as f32)
        } else {
            IrConst::F64(v)
        }
    };
    match op {
        BinOp::Add => Some(make_float(l + r)),
        BinOp::Sub => Some(make_float(l - r)),
        BinOp::Mul => Some(make_float(l * r)),
        BinOp::Div => Some(make_float(l / r)),
        BinOp::Eq => Some(IrConst::I64(if l == r { 1 } else { 0 })),
        BinOp::Ne => Some(IrConst::I64(if l != r { 1 } else { 0 })),
        BinOp::Lt => Some(IrConst::I64(if l < r { 1 } else { 0 })),
        BinOp::Gt => Some(IrConst::I64(if l > r { 1 } else { 0 })),
        BinOp::Le => Some(IrConst::I64(if l <= r { 1 } else { 0 })),
        BinOp::Ge => Some(IrConst::I64(if l >= r { 1 } else { 0 })),
        BinOp::LogicalAnd => Some(IrConst::I64(if l != 0.0 && r != 0.0 { 1 } else { 0 })),
        BinOp::LogicalOr => Some(IrConst::I64(if l != 0.0 || r != 0.0 { 1 } else { 0 })),
        _ => None,
    }
}

/// Dispatcher: like ccc's eval_const_binop, chooses the path by operand kinds.
fn eval_const_binop(op: &BinOp, lhs: &IrConst, rhs: &IrConst, is_32bit: bool, is_unsigned: bool) -> Option<IrConst> {
    if matches!(lhs, IrConst::F32(_) | IrConst::F64(_)) || matches!(rhs, IrConst::F32(_) | IrConst::F64(_)) {
        return eval_const_binop_float(op, lhs, rhs);
    }
    if matches!(lhs, IrConst::I128(_)) || matches!(rhs, IrConst::I128(_)) {
        return eval_const_binop_i128(op, lhs, rhs, is_unsigned);
    }
    eval_const_binop_int(op, lhs.to_i64()?, rhs.to_i64()?, is_32bit, is_unsigned)
}

fn show(c: &Option<IrConst>) -> String {
    match c {
        None => "none".to_string(),
        Some(IrConst::I32(v)) => format!("i32:{}", v),
        Some(IrConst::I64(v)) => format!("i64:{}", v),
        Some(IrConst::I128(v)) => format!("i128:{}", v),
        Some(IrConst::F32(v)) => format!("f32:{:?}", v),
        Some(IrConst::F64(v)) => format!("f64:{:?}", v),
        Some(other) => format!("{:?}", other),
    }
}

// --- a constant expression tree, folded bottom-up -------------------------------

#[derive(Debug)]
enum CExpr {
    Lit(IrConst),
    Bin(BinOp, Box<CExpr>, Box<CExpr>),
    Cond(Box<CExpr>, Box<CExpr>, Box<CExpr>),
}

fn fold(e: &CExpr, is_32bit: bool, is_unsigned: bool, depth: usize, trace: &mut Vec<String>) -> Option<IrConst> {
    let r = match e {
        CExpr::Lit(c) => Some(*c),
        CExpr::Bin(op, l, r) => {
            let a = fold(l, is_32bit, is_unsigned, depth + 1, trace)?;
            let b = fold(r, is_32bit, is_unsigned, depth + 1, trace)?;
            let v = eval_const_binop(op, &a, &b, is_32bit, is_unsigned);
            trace.push(format!("{}{:?} {} {} => {}", "  ".repeat(depth), op, show(&Some(a)), show(&Some(b)), show(&v)));
            v
        }
        CExpr::Cond(c, t, f) => {
            let cv = fold(c, is_32bit, is_unsigned, depth + 1, trace)?;
            let taken = match cv {
                IrConst::F32(x) => x != 0.0,
                IrConst::F64(x) => x != 0.0,
                other => other.to_i128()? != 0,
            };
            if taken { fold(t, is_32bit, is_unsigned, depth + 1, trace) } else { fold(f, is_32bit, is_unsigned, depth + 1, trace) }
        }
    };
    r
}

fn lit(v: i64) -> Box<CExpr> {
    Box::new(CExpr::Lit(IrConst::I64(v)))
}

fn bin(op: BinOp, a: Box<CExpr>, b: Box<CExpr>) -> Box<CExpr> {
    Box::new(CExpr::Bin(op, a, b))
}

fn main() {
    let ops = [
        BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div, BinOp::Mod, BinOp::BitAnd, BinOp::BitOr, BinOp::BitXor,
        BinOp::Shl, BinOp::Shr, BinOp::Eq, BinOp::Ne, BinOp::Lt, BinOp::Gt, BinOp::Le, BinOp::Ge, BinOp::LogicalAnd, BinOp::LogicalOr,
    ];
    let pairs: [(i64, i64); 8] = [
        (7, 3),
        (-7, 3),
        (i32::MAX as i64, 1),
        (i32::MIN as i64, -1),
        (-1, 31),
        (0xffff_ffff, 2),
        (i64::MIN, -1),
        (5, 0),
    ];
    for &(is_32bit, is_unsigned, label) in &[(true, false, "int"), (true, true, "unsigned"), (false, false, "long"), (false, true, "unsigned long")] {
        println!("== {}", label);
        for &(l, r) in &pairs {
            let mut line = format!("{:>20} {:>3}:", l, r);
            for op in &ops {
                line.push(' ');
                line.push_str(&show(&eval_const_binop_int(op, l, r, is_32bit, is_unsigned)));
            }
            println!("{}", line);
        }
    }
    println!("== i128");
    let big = IrConst::I128(i128::MAX);
    let neg = IrConst::I128(-(1i128 << 100));
    for op in &ops {
        println!("{:?}: {} | {} | {}", op, show(&eval_const_binop(op, &big, &IrConst::I64(2), false, false)), show(&eval_const_binop(op, &neg, &IrConst::I32(3), false, true)), show(&eval_const_binop(op, &neg, &IrConst::I64(0), false, false)));
    }
    println!("== float");
    let cases = [(IrConst::F64(1.5), IrConst::F64(-0.25)), (IrConst::F32(0.1), IrConst::F32(0.2)), (IrConst::F64(1.0), IrConst::I32(0)), (IrConst::F32(3.0), IrConst::F64(3.0))];
    for (a, b) in &cases {
        let row: Vec<String> = ops.iter().map(|op| show(&eval_const_binop(op, a, b, false, false))).collect();
        println!("{:?} {:?}: {}", a, b, row.join(" "));
    }
    println!("== folding");
    // ((1 << 31) / 3) as int, then compare with unsigned semantics etc.
    let exprs: Vec<(Box<CExpr>, bool, bool)> = vec![
        (bin(BinOp::Div, bin(BinOp::Shl, lit(1), lit(31)), lit(3)), true, false),
        (bin(BinOp::Div, bin(BinOp::Shl, lit(1), lit(31)), lit(3)), true, true),
        (bin(BinOp::Add, bin(BinOp::Mul, lit(2147483647), lit(2)), lit(1)), true, true),
        (bin(BinOp::Lt, lit(-1), lit(1)), true, true),
        (bin(BinOp::Lt, lit(-1), lit(1)), true, false),
        (Box::new(CExpr::Cond(bin(BinOp::Eq, lit(4), lit(4)), bin(BinOp::Mod, lit(-9), lit(4)), lit(0))), false, false),
        (bin(BinOp::Add, lit(1), bin(BinOp::Div, lit(1), lit(0))), false, false),
        (bin(BinOp::BitXor, Box::new(CExpr::Lit(IrConst::I128(1 << 64))), lit(-1)), false, false),
        (bin(BinOp::Mul, Box::new(CExpr::Lit(IrConst::F64(0.5))), bin(BinOp::Sub, lit(10), lit(3))), false, false),
    ];
    for (i, (e, w, u)) in exprs.iter().enumerate() {
        let mut trace = Vec::new();
        let r = fold(e, *w, *u, 0, &mut trace);
        println!("expr {} (32bit={}, unsigned={}) = {}", i, w, u, show(&r));
        for t in trace {
            println!("    {}", t);
        }
    }
}
