// A mini assembler line parser: Result-propagating helpers over &str.
enum Err {
    UnknownOp,
    BadReg(u8),
    MissingOperand,
    BadImm,
}

enum Insn {
    Mov(u8, i32),
    Add(u8, u8),
    Jmp(i32),
}

fn reg(s: &str) -> Result<u8, Err> {
    let b = s.as_bytes();
    if b.len() != 2 || b[0] != b'r' {
        return Err(Err::BadReg(if b.is_empty() { 0 } else { b[0] }));
    }
    let n = b[1].wrapping_sub(b'0');
    if n > 7 { Err(Err::BadReg(b[1])) } else { Ok(n) }
}

fn imm(s: &str) -> Result<i32, Err> {
    s.parse::<i32>().map_err(bad_imm)
}

fn bad_imm(_e: std::num::ParseIntError) -> Err {
    Err::BadImm
}

fn operand<'a>(parts: &[&'a str], i: usize) -> Result<&'a str, Err> {
    parts.get(i).copied().ok_or(Err::MissingOperand)
}

fn parse(line: &str) -> Result<Insn, Err> {
    let mut parts = [""; 3];
    let mut n = 0;
    for p in line.split(' ') {
        if !p.is_empty() && n < 3 {
            parts[n] = p;
            n += 1;
        }
    }
    let parts = &parts[..n];
    match operand(parts, 0)? {
        "mov" => Ok(Insn::Mov(reg(operand(parts, 1)?)?, imm(operand(parts, 2)?)?)),
        "add" => Ok(Insn::Add(reg(operand(parts, 1)?)?, reg(operand(parts, 2)?)?)),
        "jmp" => Ok(Insn::Jmp(imm(operand(parts, 1)?)?)),
        _ => Err(Err::UnknownOp),
    }
}

fn main() {
    let src = ["mov r1 42", "add r1 r2", "jmp -8", "mov r9 1", "add r1", "nop", "mov r0 x1", "jmp  100", "mov q1 5", ""];
    let mut regs = [0i32; 8];
    for i in 0..src.len() {
        print!("{}[{}] ", i, src[i]);
        match parse(src[i]) {
            Ok(Insn::Mov(r, v)) => {
                regs[r as usize] = v;
                println!("mov r{} {}", r, v);
            }
            Ok(Insn::Add(a, b)) => {
                regs[a as usize] += regs[b as usize] + 1;
                println!("add r{} r{} -> {}", a, b, regs[a as usize]);
            }
            Ok(Insn::Jmp(t)) => println!("jmp {}", t),
            Err(Err::UnknownOp) => println!("error: unknown op"),
            Err(Err::BadReg(c)) => println!("error: bad reg byte {}", c),
            Err(Err::MissingOperand) => println!("error: missing operand"),
            Err(Err::BadImm) => println!("error: bad immediate"),
        }
    }
}
