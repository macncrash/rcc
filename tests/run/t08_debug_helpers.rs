// Outer's fields are only read through derived Debug.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fmt;

struct Inst {
    op: &'static str,
    args: Vec<i32>,
    secret: u64,
}

impl fmt::Debug for Inst {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Inst").field("op", &self.op).field("args", &self.args).finish_non_exhaustive()?;
        let _ = self.secret;
        Ok(())
    }
}

struct Pt(i32, i32);
impl fmt::Debug for Pt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Pt").field(&self.0).field(&self.1).finish()
    }
}

struct Stack(Vec<Pt>);
impl fmt::Debug for Stack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.0.iter()).entry(&"<bottom>").finish()
    }
}

struct Env(Vec<(String, i32)>);
impl fmt::Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut m = f.debug_map();
        for (k, v) in &self.0 {
            m.entry(k, v);
        }
        m.key(&"extra").value(&Pt(0, 0));
        m.finish()
    }
}

struct Bag(Vec<char>);
impl fmt::Debug for Bag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_set().entries(&self.0).finish()
    }
}

struct Hex(u32);
impl fmt::Debug for Hex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() { write!(f, "Hex({:#x})", self.0) } else { write!(f, "0x{:x}", self.0) }
    }
}

struct Empty;
impl fmt::Debug for Empty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Empty").finish()
    }
}

#[derive(Debug)]
struct Outer {
    inst: Inst,
    stack: Stack,
    hex: Hex,
    m: BTreeMap<u8, Hex>,
    e: Empty,
}

fn main() {
    let i = Inst { op: "add", args: vec![1, 2], secret: 99 };
    println!("{:?}", i);
    println!("{:#?}", i);
    println!("{:?} {:#?}", Pt(1, -2), Pt(3, 4));
    let s = Stack(vec![Pt(1, 1), Pt(2, 2)]);
    println!("{:?}", s);
    let env = Env(vec![("x".into(), 1), ("y".into(), -1)]);
    println!("{:?}", env);
    println!("{:#?}", env);
    println!("{:?} {:#?}", Bag(vec!['a', 'b']), Bag(vec![]));
    println!("{:?} {:#?}", Hex(255), Hex(255));
    println!("{:?} {:#?}", Empty, Empty);
    let o = Outer {
        inst: Inst { op: "ret", args: vec![], secret: 0 },
        stack: Stack(vec![Pt(9, 8)]),
        hex: Hex(16),
        m: [(1, Hex(1)), (2, Hex(2))].into_iter().collect(),
        e: Empty,
    };
    println!("{:?}", o);
    println!("{:#?}", o);
    println!("{:5?}", Pt(1, 2));
}
