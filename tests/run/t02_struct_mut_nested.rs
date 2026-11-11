struct Reg {
    val: u32,
    dirty: bool,
}

struct Bank {
    regs: [Reg; 4],
    writes: u32,
}

struct Cpu {
    bank: Bank,
    pc: u32,
    flags: (bool, bool),
}

impl Cpu {
    fn write(&mut self, r: usize, v: u32) {
        let reg = &mut self.bank.regs[r];
        reg.val = v;
        reg.dirty = true;
        self.bank.writes += 1;
        self.flags.0 = v == 0;
        self.flags.1 = v > 0x7fff_ffff;
    }
    fn reg_mut(&mut self, r: usize) -> &mut Reg {
        &mut self.bank.regs[r]
    }
    fn dump(&self) {
        for i in 0..4 {
            let r = &self.bank.regs[i];
            println!("  r{} = {} dirty={}", i, r.val, r.dirty);
        }
        println!("  pc {} writes {} z={} n={}", self.pc, self.bank.writes, self.flags.0, self.flags.1);
    }
}

fn new_reg() -> Reg {
    Reg { val: 0, dirty: false }
}

fn main() {
    let mut cpu = Cpu {
        bank: Bank { regs: [new_reg(), new_reg(), new_reg(), new_reg()], writes: 0 },
        pc: 0,
        flags: (false, false),
    };
    cpu.dump();
    cpu.write(1, 5);
    cpu.write(3, 0x8000_0000);
    cpu.pc += 4;
    cpu.dump();
    {
        let r = cpu.reg_mut(2);
        r.val = 99;
    }
    cpu.reg_mut(1).val *= 3;
    cpu.write(0, 0);
    cpu.pc += 4;
    cpu.dump();
    let Cpu { bank: Bank { regs, writes }, pc, .. } = cpu;
    println!("{} {} {}", regs[2].val, writes, pc);
}
