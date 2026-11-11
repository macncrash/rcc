// Associated consts in traits and inherent impls, with defaults.
trait Unit {
    const NAME: &'static str;
    const FACTOR: u64;
    const PLURAL: bool = true;
    fn to_base(v: u64) -> u64 {
        v * Self::FACTOR
    }
}

struct Meter;
struct Km;
struct Inch;

impl Unit for Meter {
    const NAME: &'static str = "m";
    const FACTOR: u64 = 1000;
}
impl Unit for Km {
    const NAME: &'static str = "km";
    const FACTOR: u64 = 1_000_000;
    const PLURAL: bool = false;
}
impl Unit for Inch {
    const NAME: &'static str = "in";
    const FACTOR: u64 = 25;
}

fn show<U: Unit>(v: u64) {
    println!("{} {} = {} mm (plural {})", v, U::NAME, U::to_base(v), U::PLURAL);
}

struct Limits;
impl Limits {
    const MAX: i32 = 100;
    const MIN: i32 = -Self::MAX;
    const RANGE: i32 = Self::MAX - Self::MIN;
}

trait HasId {
    const ID: u32;
}
struct A;
struct B;
impl HasId for A {
    const ID: u32 = 7;
}
impl HasId for B {
    const ID: u32 = A::ID * 6;
}

fn sum_ids<X: HasId, Y: HasId>() -> u32 {
    X::ID + Y::ID
}

fn main() {
    show::<Meter>(3);
    show::<Km>(2);
    show::<Inch>(12);
    println!("{} {} {}", Limits::MAX, Limits::MIN, Limits::RANGE);
    println!("{}", sum_ids::<A, B>());
    println!("{}", sum_ids::<B, B>());
    println!("{}", <Km as Unit>::FACTOR / <Meter as Unit>::FACTOR);
    let arr = [0u8; Limits::MAX as usize];
    println!("{}", arr.len());
}
