#![allow(dead_code)]
// t12: a small stack VM with an enum instruction set, an assembler that
// resolves labels, a bytecode encoder/decoder and a disassembler.
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Push(i64),
    Pop,
    Dup,
    Swap,
    Over,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Eq,
    Lt,
    Gt,
    Not,
    Load(u8),
    Store(u8),
    Jmp(u32),
    Jz(u32),
    Jnz(u32),
    Call(u32),
    Ret,
    Print,
    Halt,
}

impl Op {
    fn opcode(&self) -> u8 {
        match self {
            Op::Push(_) => 0x01,
            Op::Pop => 0x02,
            Op::Dup => 0x03,
            Op::Swap => 0x04,
            Op::Over => 0x05,
            Op::Add => 0x10,
            Op::Sub => 0x11,
            Op::Mul => 0x12,
            Op::Div => 0x13,
            Op::Mod => 0x14,
            Op::Neg => 0x15,
            Op::And => 0x16,
            Op::Or => 0x17,
            Op::Xor => 0x18,
            Op::Shl => 0x19,
            Op::Shr => 0x1a,
            Op::Eq => 0x20,
            Op::Lt => 0x21,
            Op::Gt => 0x22,
            Op::Not => 0x23,
            Op::Load(_) => 0x30,
            Op::Store(_) => 0x31,
            Op::Jmp(_) => 0x40,
            Op::Jz(_) => 0x41,
            Op::Jnz(_) => 0x42,
            Op::Call(_) => 0x43,
            Op::Ret => 0x44,
            Op::Print => 0x50,
            Op::Halt => 0xff,
        }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        out.push(self.opcode());
        match *self {
            Op::Push(v) => out.extend_from_slice(&v.to_le_bytes()),
            Op::Load(s) | Op::Store(s) => out.push(s),
            Op::Jmp(t) | Op::Jz(t) | Op::Jnz(t) | Op::Call(t) => out.extend_from_slice(&t.to_le_bytes()),
            _ => {}
        }
    }

    fn decode(bytes: &[u8], pos: &mut usize) -> Option<Op> {
        let opc = *bytes.get(*pos)?;
        *pos += 1;
        let rd_u32 = |p: &mut usize| -> Option<u32> {
            let s = bytes.get(*p..*p + 4)?;
            *p += 4;
            Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        };
        let op = match opc {
            0x01 => {
                let s = bytes.get(*pos..*pos + 8)?;
                *pos += 8;
                let mut a = [0u8; 8];
                a.copy_from_slice(s);
                Op::Push(i64::from_le_bytes(a))
            }
            0x02 => Op::Pop,
            0x03 => Op::Dup,
            0x04 => Op::Swap,
            0x05 => Op::Over,
            0x10 => Op::Add,
            0x11 => Op::Sub,
            0x12 => Op::Mul,
            0x13 => Op::Div,
            0x14 => Op::Mod,
            0x15 => Op::Neg,
            0x16 => Op::And,
            0x17 => Op::Or,
            0x18 => Op::Xor,
            0x19 => Op::Shl,
            0x1a => Op::Shr,
            0x20 => Op::Eq,
            0x21 => Op::Lt,
            0x22 => Op::Gt,
            0x23 => Op::Not,
            0x30 | 0x31 => {
                let s = *bytes.get(*pos)?;
                *pos += 1;
                if opc == 0x30 { Op::Load(s) } else { Op::Store(s) }
            }
            0x40 => Op::Jmp(rd_u32(pos)?),
            0x41 => Op::Jz(rd_u32(pos)?),
            0x42 => Op::Jnz(rd_u32(pos)?),
            0x43 => Op::Call(rd_u32(pos)?),
            0x44 => Op::Ret,
            0x50 => Op::Print,
            0xff => Op::Halt,
            _ => return None,
        };
        Some(op)
    }
}

#[derive(Debug)]
enum VmError {
    StackUnderflow { pc: usize, op: Op },
    DivByZero { pc: usize },
    BadJump { pc: usize, target: u32 },
    StepLimit(u64),
}

struct Frame {
    ret_pc: usize,
    locals: [i64; 8],
}

struct Vm<'p> {
    prog: &'p [Op],
    stack: Vec<i64>,
    frames: Vec<Frame>,
    pc: usize,
    steps: u64,
    out: String,
    max_depth: usize,
}

impl<'p> Vm<'p> {
    fn new(prog: &'p [Op]) -> Self {
        Vm { prog, stack: Vec::new(), frames: vec![Frame { ret_pc: usize::MAX, locals: [0; 8] }], pc: 0, steps: 0, out: String::new(), max_depth: 1 }
    }

    fn pop(&mut self, op: Op) -> Result<i64, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow { pc: self.pc, op })
    }

    fn jump(&mut self, t: u32) -> Result<(), VmError> {
        if (t as usize) >= self.prog.len() {
            return Err(VmError::BadJump { pc: self.pc, target: t });
        }
        self.pc = t as usize;
        Ok(())
    }

    fn run(&mut self, limit: u64) -> Result<(), VmError> {
        loop {
            if self.steps >= limit {
                return Err(VmError::StepLimit(self.steps));
            }
            self.steps += 1;
            let op = self.prog[self.pc];
            let here = self.pc;
            self.pc += 1;
            match op {
                Op::Push(v) => self.stack.push(v),
                Op::Pop => {
                    self.pop(op)?;
                }
                Op::Dup => {
                    let v = self.pop(op)?;
                    self.stack.push(v);
                    self.stack.push(v);
                }
                Op::Swap => {
                    let b = self.pop(op)?;
                    let a = self.pop(op)?;
                    self.stack.push(b);
                    self.stack.push(a);
                }
                Op::Over => {
                    let b = self.pop(op)?;
                    let a = self.pop(op)?;
                    self.stack.extend_from_slice(&[a, b, a]);
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod | Op::And | Op::Or | Op::Xor | Op::Shl | Op::Shr | Op::Eq | Op::Lt | Op::Gt => {
                    let b = self.pop(op)?;
                    let a = self.pop(op)?;
                    let r = match op {
                        Op::Add => a.wrapping_add(b),
                        Op::Sub => a.wrapping_sub(b),
                        Op::Mul => a.wrapping_mul(b),
                        Op::Div | Op::Mod if b == 0 => return Err(VmError::DivByZero { pc: here }),
                        Op::Div => a.wrapping_div(b),
                        Op::Mod => a.wrapping_rem(b),
                        Op::And => a & b,
                        Op::Or => a | b,
                        Op::Xor => a ^ b,
                        Op::Shl => a.wrapping_shl(b as u32),
                        Op::Shr => a.wrapping_shr(b as u32),
                        Op::Eq => (a == b) as i64,
                        Op::Lt => (a < b) as i64,
                        Op::Gt => (a > b) as i64,
                        _ => unreachable!(),
                    };
                    self.stack.push(r);
                }
                Op::Neg => {
                    let a = self.pop(op)?;
                    self.stack.push(a.wrapping_neg());
                }
                Op::Not => {
                    let a = self.pop(op)?;
                    self.stack.push((a == 0) as i64);
                }
                Op::Load(s) => {
                    let v = self.frames.last().unwrap().locals[s as usize];
                    self.stack.push(v);
                }
                Op::Store(s) => {
                    let v = self.pop(op)?;
                    self.frames.last_mut().unwrap().locals[s as usize] = v;
                }
                Op::Jmp(t) => self.jump(t)?,
                Op::Jz(t) => {
                    if self.pop(op)? == 0 {
                        self.jump(t)?;
                    }
                }
                Op::Jnz(t) => {
                    if self.pop(op)? != 0 {
                        self.jump(t)?;
                    }
                }
                Op::Call(t) => {
                    // arguments: top of stack is argc, then args
                    let argc = self.pop(op)? as usize;
                    let mut f = Frame { ret_pc: self.pc, locals: [0; 8] };
                    for i in (0..argc).rev() {
                        f.locals[i] = self.pop(op)?;
                    }
                    self.frames.push(f);
                    self.max_depth = self.max_depth.max(self.frames.len());
                    self.jump(t)?;
                }
                Op::Ret => {
                    let f = self.frames.pop().unwrap();
                    self.pc = f.ret_pc;
                }
                Op::Print => {
                    let v = self.pop(op)?;
                    writeln!(self.out, "{}", v).unwrap();
                }
                Op::Halt => return Ok(()),
            }
        }
    }
}

// --- assembler with symbolic labels ----------------------------------------

enum Item {
    Ins(Op),
    Label(&'static str),
    JmpL(&'static str),
    JzL(&'static str),
    JnzL(&'static str),
    CallL(&'static str),
}

fn assemble(items: &[Item]) -> Result<Vec<Op>, String> {
    let mut labels: HashMap<&str, u32> = HashMap::new();
    let mut n = 0u32;
    for it in items {
        match it {
            Item::Label(l) => {
                if labels.insert(l, n).is_some() {
                    return Err(format!("duplicate label {}", l));
                }
            }
            _ => n += 1,
        }
    }
    let resolve = |l: &str| labels.get(l).copied().ok_or_else(|| format!("undefined label {}", l));
    let mut out = Vec::new();
    for it in items {
        let op = match it {
            Item::Ins(op) => *op,
            Item::Label(_) => continue,
            Item::JmpL(l) => Op::Jmp(resolve(l)?),
            Item::JzL(l) => Op::Jz(resolve(l)?),
            Item::JnzL(l) => Op::Jnz(resolve(l)?),
            Item::CallL(l) => Op::Call(resolve(l)?),
        };
        out.push(op);
    }
    let mut sorted: Vec<_> = labels.into_iter().collect();
    sorted.sort_by_key(|&(name, pc)| (pc, name));
    for (name, pc) in sorted {
        println!("  label {:<8} = {}", name, pc);
    }
    Ok(out)
}

fn disasm(prog: &[Op]) -> String {
    let mut s = String::new();
    for (i, op) in prog.iter().enumerate() {
        let _ = match op {
            Op::Push(v) => writeln!(s, "{:04} push {}", i, v),
            Op::Load(n) => writeln!(s, "{:04} load l{}", i, n),
            Op::Store(n) => writeln!(s, "{:04} store l{}", i, n),
            Op::Jmp(t) | Op::Jz(t) | Op::Jnz(t) | Op::Call(t) => {
                let name = format!("{:?}", op);
                let mnem = name.split('(').next().unwrap().to_lowercase();
                writeln!(s, "{:04} {} @{}", i, mnem, t)
            }
            other => writeln!(s, "{:04} {}", i, format!("{:?}", other).to_lowercase()),
        };
    }
    s
}

fn hexdump(bytes: &[u8]) {
    for (row, chunk) in bytes.chunks(16).enumerate() {
        let mut line = format!("{:06x}:", row * 16);
        for b in chunk {
            write!(line, " {:02x}", b).unwrap();
        }
        println!("{}", line);
    }
}

fn run_program(name: &str, items: &[Item], limit: u64) {
    println!("== {}", name);
    let prog = match assemble(items) {
        Ok(p) => p,
        Err(e) => {
            println!("  asm error: {}", e);
            return;
        }
    };
    let mut bytes = Vec::new();
    for op in &prog {
        op.encode(&mut bytes);
    }
    println!("  {} ops, {} bytes", prog.len(), bytes.len());
    // round trip through bytecode
    let mut pos = 0;
    let mut decoded = Vec::new();
    while pos < bytes.len() {
        decoded.push(Op::decode(&bytes, &mut pos).expect("decode"));
    }
    println!("  roundtrip equal: {}", decoded == prog);
    let mut vm = Vm::new(&decoded);
    let r = vm.run(limit);
    print!("{}", vm.out);
    println!("  result: {:?} steps={} depth={} stack={:?}", r, vm.steps, vm.max_depth, vm.stack);
}

fn main() {
    use Item::*;
    use Op::*;
    // factorial loop: l0 = n, l1 = acc
    let fact = [
        Ins(Push(10)), Ins(Store(0)), Ins(Push(1)), Ins(Store(1)),
        Label("loop"),
        Ins(Load(0)), JzL("done"),
        Ins(Load(1)), Ins(Load(0)), Ins(Mul), Ins(Store(1)),
        Ins(Load(0)), Ins(Push(1)), Ins(Sub), Ins(Store(0)),
        Ins(Load(1)), Ins(Print),
        JmpL("loop"),
        Label("done"),
        Ins(Load(1)), Ins(Print), Ins(Halt),
    ];
    run_program("fact", &fact, 10_000);
    let prog = assemble(&fact).unwrap();
    print!("{}", disasm(&prog));
    let mut bytes = Vec::new();
    for op in prog.iter().take(6) {
        op.encode(&mut bytes);
    }
    hexdump(&bytes);

    // recursive fib via Call: fib(n) in l0
    let fib = [
        Ins(Push(0)), Store(0).into_item(),
        Label("main_loop"),
        Ins(Load(0)), Ins(Push(15)), Ins(Lt), JzL("end"),
        Ins(Load(0)), Ins(Push(1)), CallL("fib"), Ins(Print),
        Ins(Load(0)), Ins(Push(1)), Ins(Add), Ins(Store(0)),
        JmpL("main_loop"),
        Label("end"), Ins(Halt),
        Label("fib"),
        Ins(Load(0)), Ins(Push(2)), Ins(Lt), JzL("rec"),
        Ins(Load(0)), Ins(Ret),
        Label("rec"),
        Ins(Load(0)), Ins(Push(1)), Ins(Sub), Ins(Push(1)), CallL("fib"),
        Ins(Load(0)), Ins(Push(2)), Ins(Sub), Ins(Push(1)), CallL("fib"),
        Ins(Add), Ins(Ret),
    ];
    run_program("fib", &fib, 1_000_000);

    // bit ops and stack shuffles
    let bits = [
        Ins(Push(0x0f0f)), Ins(Push(0x00ff)), Ins(Over), Ins(Over), Ins(And), Ins(Print),
        Ins(Over), Ins(Over), Ins(Or), Ins(Print),
        Ins(Xor), Ins(Print),
        Ins(Push(1)), Ins(Push(62)), Ins(Shl), Ins(Dup), Ins(Print), Ins(Push(2)), Ins(Mul), Ins(Print),
        Ins(Push(-17)), Ins(Push(2)), Ins(Shr), Ins(Print),
        Ins(Push(-17)), Ins(Push(5)), Ins(Mod), Ins(Print),
        Ins(Push(-17)), Ins(Push(5)), Ins(Div), Ins(Neg), Ins(Print),
        Ins(Push(3)), Ins(Push(4)), Ins(Swap), Ins(Sub), Ins(Print),
        Ins(Push(5)), Ins(Push(5)), Ins(Eq), Ins(Not), Ins(Print),
        Ins(Push(9)), Ins(Push(3)), Ins(Gt), Ins(Print),
        Ins(Halt),
    ];
    run_program("bits", &bits, 1000);

    // sieve-like: count numbers < 200 with popcount parity even, using Jnz
    let parity = [
        Ins(Push(0)), Ins(Store(0)), Ins(Push(0)), Ins(Store(2)),
        Label("outer"),
        Ins(Load(0)), Ins(Push(200)), Ins(Lt), JzL("fin"),
        Ins(Load(0)), Ins(Store(1)), Ins(Push(0)), Ins(Store(3)),
        Label("inner"),
        Ins(Load(1)), JzL("inner_done"),
        Ins(Load(3)), Ins(Load(1)), Ins(Push(1)), Ins(And), Ins(Xor), Ins(Store(3)),
        Ins(Load(1)), Ins(Push(1)), Ins(Shr), Ins(Store(1)),
        JmpL("inner"),
        Label("inner_done"),
        Ins(Load(3)), JnzL("skip"),
        Ins(Load(2)), Ins(Push(1)), Ins(Add), Ins(Store(2)),
        Label("skip"),
        Ins(Load(0)), Ins(Push(1)), Ins(Add), Ins(Store(0)),
        JmpL("outer"),
        Label("fin"), Ins(Load(2)), Ins(Print), Ins(Halt),
    ];
    run_program("parity", &parity, 100_000);

    // error cases
    run_program("underflow", &[Ins(Push(1)), Ins(Add), Ins(Halt)], 100);
    run_program("divzero", &[Ins(Push(1)), Ins(Push(0)), Ins(Div), Ins(Halt)], 100);
    run_program("infinite", &[Label("x"), JmpL("x")], 500);
    run_program("badlabel", &[JmpL("nowhere")], 10);
    run_program("duplabel", &[Label("a"), Label("a"), Ins(Halt)], 10);
    let mut pos = 0;
    println!("decode bad: {:?}", Op::decode(&[0x99], &mut pos));
    pos = 0;
    println!("decode short: {:?}", Op::decode(&[0x01, 1, 2], &mut pos));
}

trait IntoItem {
    fn into_item(self) -> Item;
}

impl IntoItem for Op {
    fn into_item(self) -> Item {
        Item::Ins(self)
    }
}
