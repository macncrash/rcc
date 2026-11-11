// derive(Hash) with a recording, deterministic hand-written Hasher.
// Shows exactly what each derived Hash impl feeds the hasher. This pins std
// details: enum discriminants are hashed as isize, bool as u8, arrays/slices
// write a usize length prefix and [u8] goes through write().
use std::hash::{Hash, Hasher};

struct Recorder {
    log: Vec<String>,
    state: u64,
}

impl Recorder {
    fn new() -> Self {
        Recorder { log: Vec::new(), state: 0 }
    }
    fn mix(&mut self, v: u64) {
        self.state = (self.state.rotate_left(5) ^ v).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

impl Hasher for Recorder {
    fn finish(&self) -> u64 {
        self.state
    }
    fn write(&mut self, bytes: &[u8]) {
        self.log.push(format!("bytes{:?}", bytes));
        for b in bytes {
            self.mix(*b as u64);
        }
    }
    fn write_u8(&mut self, i: u8) {
        self.log.push(format!("u8 {}", i));
        self.mix(i as u64);
    }
    fn write_u32(&mut self, i: u32) {
        self.log.push(format!("u32 {}", i));
        self.mix(i as u64);
    }
    fn write_u64(&mut self, i: u64) {
        self.log.push(format!("u64 {}", i));
        self.mix(i);
    }
    fn write_i32(&mut self, i: i32) {
        self.log.push(format!("i32 {}", i));
        self.mix(i as u64);
    }
    fn write_usize(&mut self, i: usize) {
        self.log.push(format!("usize {}", i));
        self.mix(i as u64);
    }
    fn write_isize(&mut self, i: isize) {
        self.log.push(format!("isize {}", i));
        self.mix(i as u64);
    }
}

#[derive(Hash)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Hash)]
enum Shape {
    Dot,
    Circle(u32),
    Line { a: Point, b: Point },
}

#[derive(Hash)]
struct Mixed {
    flag: bool,
    id: u64,
    data: [u8; 2],
}

struct Custom(u32, u32);
impl Hash for Custom {
    fn hash<H: Hasher>(&self, h: &mut H) {
        // only the first field participates
        self.0.hash(h);
    }
}

fn show<T: Hash>(label: &str, t: &T) {
    let mut r = Recorder::new();
    t.hash(&mut r);
    println!("{}: {} -> {:016x}", label, r.log.join(", "), r.finish());
}

fn main() {
    show("point", &Point { x: 1, y: -2 });
    show("dot", &Shape::Dot);
    show("circle", &Shape::Circle(9));
    show("line", &Shape::Line { a: Point { x: 0, y: 1 }, b: Point { x: 2, y: 3 } });
    show("mixed", &Mixed { flag: true, id: 77, data: [5, 6] });
    show("custom", &Custom(4, 5));
    show("custom2", &Custom(4, 999));
    let c = Custom(1, 2);
    println!("unhashed field {}", c.1);
    show("tuple", &(1u8, 2u32));
    show("u64", &12345u64);
    let mut a = Recorder::new();
    let mut b = Recorder::new();
    Point { x: 3, y: 4 }.hash(&mut a);
    Point { x: 3, y: 4 }.hash(&mut b);
    println!("equal hashes: {}", a.finish() == b.finish());
    let mut c = Recorder::new();
    Point { x: 4, y: 3 }.hash(&mut c);
    println!("swapped differs: {}", a.finish() != c.finish());
}
