// Default: derived, hand-written, struct update syntax, generic T::default().
#![allow(dead_code)] // fields are only observed through Debug
#[derive(Debug, Default)]
struct Config {
    verbose: bool,
    level: u8,
    name: String,
    ratio: f64,
    tag: Option<char>,
    counts: [u16; 3],
}

#[derive(Debug)]
struct Limits {
    lo: i32,
    hi: i32,
}
impl Default for Limits {
    fn default() -> Self {
        Limits { lo: -10, hi: 10 }
    }
}

#[derive(Debug, Default)]
struct Outer {
    limits: Limits,
    id: u64,
    pair: (i8, bool),
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum Mode {
    Fast,
    #[default]
    Normal,
    Slow,
}

fn make<T: Default>() -> T {
    T::default()
}

fn or_default<T: Default + Copy>(o: Option<T>) -> T {
    match o {
        Some(v) => v,
        None => Default::default(),
    }
}

fn main() {
    let c = Config::default();
    println!("{:?}", c);
    let c2 = Config { level: 3, name: String::from("x"), ..Default::default() };
    println!("{:?}", c2);
    let o: Outer = Default::default();
    println!("{:?}", o);
    let o2 = Outer { id: 9, ..Outer::default() };
    println!("{} {} {}", o2.id, o2.limits.lo, o2.limits.hi);
    println!("{:?}", Mode::default());
    println!("{}", Mode::default() == Mode::Normal);
    println!("{:?} {:?}", Mode::Fast, Mode::Slow);
    let i: i32 = make();
    let s: String = make();
    let b: bool = make();
    let t: (u8, char, f32) = make();
    println!("{} {:?} {} {:?}", i, s, b, t);
    println!("{}", or_default(Some(5u32)));
    println!("{}", or_default::<u32>(None));
    println!("{:?}", or_default::<Mode>(None));
    let v: Vec<i32> = Default::default();
    println!("{}", v.len());
}
