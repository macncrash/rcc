// macro_rules! basics: multiple arms, expr fragments, literal arms, precedence.
macro_rules! square {
    ($x:expr) => {
        $x * $x
    };
}

macro_rules! op {
    (add $a:expr, $b:expr) => { $a + $b };
    (sub $a:expr, $b:expr) => { $a - $b };
    (neg $a:expr) => { -$a };
    ($a:expr) => { $a };
}

macro_rules! max_of {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => {{
        let a = $x;
        let b = max_of!($($rest),+);
        if a > b { a } else { b }
    }};
}

macro_rules! unit {
    () => {
        "empty"
    };
}

macro_rules! calls {
    ($f:expr) => {{
        let mut n = 0;
        let r = $f(&mut n);
        (r, n)
    }};
}

fn bump(n: &mut i32) -> i32 {
    *n += 5;
    *n * 2
}

fn main() {
    // expr fragment keeps grouping: square!(1 + 2) == 9, not 5
    println!("{}", square!(1 + 2));
    println!("{}", square!(7));
    println!("{}", op!(add 3, 4));
    println!("{}", op!(sub 10, op!(add 1, 2)));
    println!("{}", op!(neg 5 - 8));
    println!("{}", op!(42));
    println!("{}", max_of!(3));
    println!("{}", max_of!(3, 9, 2, 8));
    println!("{}", max_of!(-1, -7));
    println!("{}", unit!());
    println!("{:?}", calls!(bump));
    // macros may be invoked with (), [] or {}
    let a = square![4];
    let b = square! {5};
    println!("{} {}", a, b);
    let mut counter = 0;
    let v = square!({ counter += 1; counter + 1 });
    println!("side effects evaluated twice: v={} counter={}", v, counter);
}
