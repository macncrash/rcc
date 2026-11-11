enum Temp {
    C(i32),
    F(i32),
}

fn feel(t: &Temp) -> &'static str {
    match *t {
        Temp::C(c) if c > 30 => "hot",
        Temp::C(c) if c < 5 => "cold",
        Temp::C(_) => "mild",
        Temp::F(f) if f > 86 => "hot",
        Temp::F(f) if f < 41 => "cold",
        Temp::F(_) => "mild",
    }
}

fn fizz(n: u32) -> u32 {
    match n {
        x if x % 15 == 0 => 15,
        x if x % 5 == 0 => 5,
        x if x % 3 == 0 => 3,
        _ => 0,
    }
}

fn check(calls: &mut i32, x: i32) -> bool {
    *calls += 1;
    x > 2
}

fn main() {
    let ts = [Temp::C(35), Temp::C(0), Temp::C(20), Temp::F(100), Temp::F(32), Temp::F(60)];
    for i in 0..ts.len() {
        println!("{}", feel(&ts[i]));
    }
    let mut sum = 0;
    for n in 1..=30 {
        let f = fizz(n);
        sum += f;
        if f != 0 {
            println!("{} -> {}", n, f);
        }
    }
    println!("sum {}", sum);
    let mut calls = 0;
    let r = match 5 {
        1 | 5 if check(&mut calls, 1) => "first",
        5 if check(&mut calls, 5) => "second",
        _ => "none",
    };
    println!("{} {}", r, calls);
    let pair = (3, 3);
    let r2 = match pair {
        (x, y) if x == y => x * 100,
        (x, y) => x + y,
    };
    println!("{}", r2);
}
