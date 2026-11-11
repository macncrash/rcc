// fn pointers: as values, in arrays/tables, coerced from non-capturing closures,
// method paths as fn pointers, comparison with Option<fn>.
fn add(a: i64, b: i64) -> i64 {
    a + b
}
fn sub(a: i64, b: i64) -> i64 {
    a - b
}
fn mul(a: i64, b: i64) -> i64 {
    a * b
}

struct Op {
    name: &'static str,
    f: fn(i64, i64) -> i64,
}

const TABLE: [Op; 3] = [
    Op { name: "add", f: add },
    Op { name: "sub", f: sub },
    Op { name: "mul", f: mul },
];

fn apply(f: fn(i64, i64) -> i64, a: i64, b: i64) -> i64 {
    f(a, b)
}

fn pick(name: &str) -> Option<fn(i64, i64) -> i64> {
    for op in TABLE.iter() {
        if op.name == name {
            return Some(op.f);
        }
    }
    None
}

struct Num(i32);
impl Num {
    fn get(&self) -> i32 {
        self.0
    }
    fn make(v: i32) -> Num {
        Num(v)
    }
}

fn map_all(xs: &[i32], f: fn(i32) -> i32) -> Vec<i32> {
    let mut v = Vec::new();
    for &x in xs {
        v.push(f(x));
    }
    v
}

fn fold(xs: &[i64], init: i64, f: fn(i64, i64) -> i64) -> i64 {
    let mut acc = init;
    for &x in xs {
        acc = f(acc, x);
    }
    acc
}

fn main() {
    for op in TABLE.iter() {
        println!("{} {}", op.name, (op.f)(12, 5));
    }
    println!("{}", apply(add, 1, 2));
    let f: fn(i64, i64) -> i64 = |a, b| a % b;
    println!("{}", apply(f, 17, 5));
    match pick("mul") {
        Some(g) => println!("mul {}", g(6, 7)),
        None => println!("none"),
    }
    println!("{}", pick("div").is_none());
    let mk: fn(i32) -> Num = Num::make;
    let get: fn(&Num) -> i32 = Num::get;
    println!("{}", get(&mk(55)));
    println!("{:?}", map_all(&[1, 2, 3], |x| x * x));
    println!("{:?}", map_all(&[-4, 4], i32::abs));
    println!("{:?}", map_all(&[1, 2], i32::wrapping_neg));
    println!("{}", fold(&[1, 2, 3, 4], 0, add));
    println!("{}", fold(&[1, 2, 3, 4], 1, mul));
    let mut fp: fn(i64, i64) -> i64 = add;
    println!("{}", fp(1, 1));
    fp = sub;
    println!("{}", fp(1, 1));
    let unops: [fn(u32) -> u32; 4] = [u32::count_ones, u32::leading_zeros, |x| x + 1, u32::reverse_bits];
    for u in unops {
        print!("{} ", u(6));
    }
    println!();
    let f2: fn() -> &'static str = || "unit fn ptr";
    println!("{}", f2());
}
