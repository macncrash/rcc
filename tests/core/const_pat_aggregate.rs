#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Quals(u8);
impl Quals {
    const NONE: Quals = Quals(0);
    const CONST: Quals = Quals(1);
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Reg { class: u8, num: u16 }
const SP: Reg = Reg { class: 1, num: 31 };
const FP: Reg = Reg { class: 1, num: 29 };
#[derive(PartialEq, Eq, Clone, Copy)]
enum Kind { A, B(u32), C { x: i8 } }
const KB: Kind = Kind::B(7);
const KC: Kind = Kind::C { x: -1 };
const PAIR: (u32, bool) = (3, true);
const ARR: [u8; 3] = [1, 2, 3];
const NAME: &str = "hello";
const OPT: Option<u32> = Some(5);
const F: f64 = 1.5;
fn q(x: Quals) -> &'static str {
    match x { Quals::NONE => "none", Quals::CONST => "const", _ => "other" }
}
fn r(x: Reg) -> u32 { match x { SP => 1, FP => 2, Reg { class: 0, .. } => 3, _ => 4 } }
fn k(x: &Kind) -> u32 { match x { &KB => 1, &KC => 2, Kind::A => 3, _ => 4 } }
fn k2(x: &Kind) -> u32 { match *x { KB => 1, KC => 2, Kind::A => 3, _ => 4 } }
fn p(x: (u32, bool)) -> u32 { match x { PAIR => 1, (_, true) => 2, _ => 3 } }
fn a(x: [u8; 3]) -> u32 { match x { ARR => 1, [0, ..] => 2, _ => 3 } }
fn s(x: &str) -> u32 { match x { NAME => 1, "x" => 2, _ => 3 } }
fn o(x: Option<u32>) -> u32 { match x { OPT => 1, Some(_) => 2, None => 3 } }
fn f(x: f64) -> u32 { match x { F => 1, _ => 2 } }
fn nested(x: Option<(Reg, Quals)>) -> u32 {
    match x { Some((SP, Quals::NONE)) => 1, Some((_, Quals::CONST)) => 2, Some(_) => 3, None => 4 }
}
fn main() {
    println!("{} {} {}", q(Quals(0)), q(Quals(1)), q(Quals(2)));
    println!("{} {} {} {}", r(SP), r(FP), r(Reg { class: 0, num: 1 }), r(Reg { class: 2, num: 1 }));
    println!("{} {} {} {}", k(&Kind::B(7)), k(&Kind::C { x: -1 }), k(&Kind::A), k(&Kind::B(8)));
    println!("{} {} {} {}", k2(&Kind::B(7)), k2(&Kind::C { x: -1 }), k2(&Kind::A), k2(&Kind::C { x: 1 }));
    println!("{} {} {}", p((3, true)), p((4, true)), p((3, false)));
    println!("{} {} {}", a([1, 2, 3]), a([0, 2, 3]), a([1, 2, 4]));
    println!("{} {} {}", s("hello"), s("x"), s("y"));
    println!("{} {} {}", o(Some(5)), o(Some(6)), o(None));
    println!("{} {}", f(1.5), f(2.5));
    println!("{} {} {} {}", nested(Some((SP, Quals::NONE))), nested(Some((FP, Quals::CONST))), nested(Some((FP, Quals(9)))), nested(None));
    if let Quals::CONST = Quals(1) { println!("iflet ok"); }
    let v = Quals(0);
    println!("{}", matches!(v, Quals::NONE | Quals::CONST));
}
