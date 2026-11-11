fn classify(n: i32) -> &'static str {
    match n {
        0 => "zero",
        1 => "one",
        2 | 3 | 5 | 7 => "small prime",
        4 | 6 | 8 | 9 => "small composite",
        -1 => "minus one",
        _ => "other",
    }
}

fn ch(c: char) -> u32 {
    match c {
        'a' => 1,
        'b' | 'c' => 2,
        'z' => 26,
        _ => 0,
    }
}

fn b(x: bool, y: bool) -> &'static str {
    match (x, y) {
        (true, true) => "both",
        (true, false) => "first",
        (false, true) => "second",
        (false, false) => "neither",
    }
}

fn s(x: &str) -> i32 {
    match x {
        "hello" => 1,
        "world" => 2,
        "" => 3,
        _ => 4,
    }
}

fn main() {
    for n in -2..11 {
        println!("{} {}", n, classify(n));
    }
    let vals = ['a', 'b', 'c', 'd', 'z'];
    for k in 0..vals.len() {
        let c = vals[k];
        println!("{} {}", c, ch(c));
    }
    println!("{} {} {} {}", b(true, true), b(true, false), b(false, true), b(false, false));
    println!("{} {} {} {}", s("hello"), s("world"), s(""), s("x"));
    let big: u64 = 1 << 40;
    let r = match big {
        1099511627776 => "2^40",
        _ => "no",
    };
    println!("{}", r);
    let v: i8 = -128;
    println!("{}", match v { -128 => "min", 127 => "max", _ => "mid" });
}
