// Associated types: a container trait and a state-machine trait.
trait Container {
    type Item;
    fn get(&self, i: usize) -> Option<Self::Item>;
    fn first(&self) -> Option<Self::Item> {
        self.get(0)
    }
    fn size(&self) -> usize;
}

struct Digits {
    n: u64,
}

impl Container for Digits {
    type Item = u8;
    fn get(&self, i: usize) -> Option<u8> {
        let mut n = self.n;
        let mut k = 0;
        while k < i {
            n /= 10;
            if n == 0 {
                return None;
            }
            k += 1;
        }
        Some((n % 10) as u8)
    }
    fn size(&self) -> usize {
        let mut n = self.n;
        let mut c = 1;
        while n >= 10 {
            n /= 10;
            c += 1;
        }
        c
    }
}

struct Letters {
    s: &'static str,
}

impl Container for Letters {
    type Item = char;
    fn get(&self, i: usize) -> Option<char> {
        self.s.chars().nth(i)
    }
    fn size(&self) -> usize {
        self.s.len()
    }
}

fn last_item<C: Container>(c: &C) -> Option<C::Item> {
    if c.size() == 0 { None } else { c.get(c.size() - 1) }
}

trait Machine {
    type State: Copy;
    type Input;
    fn init(&self) -> Self::State;
    fn step(&self, s: Self::State, i: Self::Input) -> Self::State;
}

struct Parity;
impl Machine for Parity {
    type State = bool;
    type Input = u32;
    fn init(&self) -> bool {
        false
    }
    fn step(&self, s: bool, i: u32) -> bool {
        s ^ (i % 2 == 1)
    }
}

fn run_machine<M: Machine<Input = u32>>(m: &M, inputs: &[u32]) -> M::State {
    let mut s = m.init();
    for &i in inputs {
        s = m.step(s, i);
    }
    s
}

fn main() {
    let d = Digits { n: 90817 };
    println!("{}", d.size());
    for i in 0..6 {
        match d.get(i) {
            Some(v) => println!("digit {} = {}", i, v),
            None => println!("digit {} none", i),
        }
    }
    println!("{:?} {:?}", d.first(), last_item(&d));
    let l = Letters { s: "hello" };
    println!("{:?} {:?} {:?}", l.first(), l.get(1), last_item(&l));
    let e = Letters { s: "" };
    println!("{:?}", last_item(&e));
    println!("{}", run_machine(&Parity, &[1, 2, 3]));
    println!("{}", run_machine(&Parity, &[1, 2, 3, 5]));
    let s: <Parity as Machine>::State = Parity.init();
    println!("{}", s);
}
