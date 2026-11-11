fn quadrant(p: (i32, i32)) -> i32 {
    match p {
        (0, 0) => 0,
        (x, 0) if x > 0 => 10,
        (_, 0) => 11,
        (0, _) => 12,
        (x, y) if x > 0 && y > 0 => 1,
        (x, y) if x < 0 && y > 0 => 2,
        (_, y) if y < 0 => {
            if p.0 < 0 { 3 } else { 4 }
        }
        _ => 99,
    }
}

fn fold3(t: (u8, (u16, u32), bool)) -> u64 {
    let (a, (b, c), d) = t;
    if d {
        a as u64 + b as u64 + c as u64
    } else {
        (a as u64) * (b as u64) * (c as u64)
    }
}

fn swap(t: (i32, i32)) -> (i32, i32) {
    let (a, b) = t;
    (b, a)
}

fn main() {
    let pts = [(0, 0), (3, 0), (-3, 0), (0, 5), (1, 1), (-1, 1), (-1, -1), (1, -1)];
    for k in 0..pts.len() {
        let p = pts[k];
        println!("({}, {}) -> {}", p.0, p.1, quadrant(p));
    }
    println!("{}", fold3((2, (3, 4), true)));
    println!("{}", fold3((2, (3, 4), false)));
    let (x, y) = swap((7, 8));
    println!("{} {}", x, y);
    let (first, .., last) = (1, 2, 3, 4, 5);
    println!("{} {}", first, last);
    let (_, second, ..) = (10, 20, 30);
    println!("{}", second);
    let nested = ((1, 2), [3, 4], (5,));
    let ((a, _), [_, d], (e,)) = nested;
    println!("{} {} {}", a, d, e);
    let mut t = (0, 0);
    for i in 0..5 {
        t = match t {
            (a, b) if a <= b => (a + i, b),
            (a, b) => (a, b + i * 2),
        };
        println!("{} {}", t.0, t.1);
    }
}
