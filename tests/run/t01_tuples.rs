fn swap(p: (i32, i32)) -> (i32, i32) {
    (p.1, p.0)
}

fn divmod(a: u32, b: u32) -> (u32, u32) {
    (a / b, a % b)
}

fn minmax(a: i64, b: i64, c: i64) -> (i64, i64) {
    let mut lo = a;
    let mut hi = a;
    if b < lo { lo = b; }
    if c < lo { lo = c; }
    if b > hi { hi = b; }
    if c > hi { hi = c; }
    (lo, hi)
}

fn main() {
    let t = (1, 2.5, 'c', true, 99u8);
    println!("{} {} {} {} {}", t.0, t.1, t.2, t.3, t.4);
    let (a, b) = swap((10, 20));
    println!("{} {}", a, b);
    let (q, r) = divmod(100, 7);
    println!("q={} r={}", q, r);
    let mm = minmax(5, -3, 12);
    println!("min={} max={}", mm.0, mm.1);
    let mut m = (0, 0);
    m.0 += 5;
    m.1 -= 5;
    println!("{} {}", m.0, m.1);
    let (x, _, z) = (1, 2, 3);
    println!("{} {}", x, z);
    let (first, ..) = (7, 8, 9, 10);
    println!("{}", first);
    let (.., last) = (7, 8, 9, 10);
    println!("{}", last);
    let unit = ();
    let () = unit;
    let single = (42,);
    println!("{}", single.0);
    let mut acc = (0u64, 1u64);
    let mut i = 0;
    while i < 15 {
        acc = (acc.1, acc.0 + acc.1);
        i += 1;
    }
    println!("fib via tuple = {}", acc.0);
    let eq = (1, 'a') == (1, 'a');
    let lt = (1, 5) < (2, 0);
    let lt2 = (1, 5) < (1, 3);
    println!("{} {} {}", eq, lt, lt2);
    let (mut p, mut s) = (1, 0);
    p *= 3;
    s += p;
    println!("{} {}", p, s);
}
