struct Meters(f64);
struct Pair(i64, u8);
struct Unit;
struct Wrapper(Pair);

impl Unit {
    fn name(&self) -> &'static str {
        "unit"
    }
}

fn double(m: Meters) -> Meters {
    Meters(m.0 * 2.0)
}

fn main() {
    let m = Meters(1.25);
    let d = double(m);
    println!("{}", d.0);
    let mut p = Pair(-9, 200);
    println!("{} {}", p.0, p.1);
    p.0 *= 3;
    p.1 = p.1.wrapping_add(100);
    println!("{} {}", p.0, p.1);
    let Pair(a, b) = p;
    println!("{} {}", a, b);
    let u = Unit;
    println!("{}", u.name());
    let u2 = Unit {};
    println!("{}", u2.name());
    let w = Wrapper(Pair(5, 6));
    println!("{} {}", (w.0).0, w.0 .1);
    let Wrapper(Pair(x, y)) = w;
    println!("{}", x + y as i64);
    let size = std::mem::size_of::<Unit>();
    println!("unit size {}", size);
    let f = Pair;
    let made = f(77, 7);
    println!("ctor as fn {} {}", made.0, made.1);
}
