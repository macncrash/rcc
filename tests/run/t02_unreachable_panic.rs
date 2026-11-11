enum Op {
    Inc,
    Dec,
    Halt,
}

fn decode(b: u8) -> Op {
    match b {
        0 => Op::Inc,
        1 => Op::Dec,
        2 => Op::Halt,
        _ => unreachable!("bad opcode {}", b),
    }
}

fn main() {
    let prog = [0u8, 0, 1, 0, 0, 2, 0, 7];
    let mut acc = 0i32;
    let mut pc = 0;
    loop {
        let op = decode(prog[pc]);
        pc += 1;
        match op {
            Op::Inc => acc += 1,
            Op::Dec => acc -= 1,
            Op::Halt => {
                println!("halt at {} acc {}", pc, acc);
                break;
            }
        }
        println!("pc {} acc {}", pc, acc);
    }
    // continue past halt: hits the bad opcode and panics (exit 101)
    loop {
        let op = decode(prog[pc]);
        pc += 1;
        if let Op::Inc = op {
            acc += 1;
        }
        println!("pc {} acc {}", pc, acc);
    }
}
