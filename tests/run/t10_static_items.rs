// statics and consts: static mut counters, static structs/slices/str tables, associated
// consts, consts referring to consts, address-independent use of &'static data.
#[derive(Debug)]
struct Reg {
    name: &'static str,
    num: u8,
}

static REGS: [Reg; 4] = [
    Reg { name: "rax", num: 0 },
    Reg { name: "rcx", num: 1 },
    Reg { name: "rdx", num: 2 },
    Reg { name: "rbx", num: 3 },
];
static KEYWORDS: &[&str] = &["if", "else", "while", "return"];
static GREETING: &str = "hello";
const MASK: u32 = (1 << BITS) - 1;
const BITS: u32 = 12;
static mut CALLS: u64 = 0;
static mut HISTORY: [i32; 4] = [0; 4];

struct Limits;
impl Limits {
    const MAX: usize = 3;
    const NAMES: [&'static str; 3] = ["lo", "mid", "hi"];
}

trait HasId {
    const ID: u32;
    fn id_plus(&self, n: u32) -> u32 {
        Self::ID + n
    }
}
struct A;
struct B;
impl HasId for A {
    const ID: u32 = 100;
}
impl HasId for B {
    const ID: u32 = 200;
}

fn record(v: i32) {
    // static mut access requires unsafe; no references are taken
    unsafe {
        HISTORY[(CALLS % 4) as usize] = v;
        CALLS += 1;
    }
}

fn lookup(name: &str) -> Option<&'static Reg> {
    REGS.iter().find(|r| r.name == name)
}

fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

fn main() {
    for r in REGS.iter() {
        println!("{} = {}", r.name, r.num);
    }
    println!("{:?}", lookup("rdx"));
    println!("{:?}", lookup("r99").map(|r| r.num));
    for w in ["if", "foo", "return"] {
        println!("{} keyword? {}", w, is_keyword(w));
    }
    println!("{} {}", GREETING, GREETING.len());
    println!("MASK = {} BITS = {}", MASK, BITS);
    for i in 1..=6 {
        record(i * 10);
    }
    let (calls, hist) = unsafe { (CALLS, HISTORY) };
    println!("calls {} history {:?}", calls, hist);
    println!("{} {:?} {}", Limits::MAX, Limits::NAMES, Limits::NAMES[Limits::MAX - 1]);
    println!("{} {} {} {}", A::ID, B::ID, A.id_plus(1), B.id_plus(2));
    let arr = [0u8; Limits::MAX * 2];
    println!("arr len {}", arr.len());
}
