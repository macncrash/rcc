fn side(name: i32, v: bool) -> bool {
    println!("eval {}", name);
    v
}

fn main() {
    let t = true;
    let f = false;
    println!("{} {}", t, f);
    println!("{} {} {} {}", t && t, t && f, f && t, f && f);
    println!("{} {} {} {}", t || t, t || f, f || t, f || f);
    println!("{} {}", !t, !f);
    println!("{} {} {}", t ^ f, t ^ t, f ^ f);
    println!("{} {}", t & f, t | f);
    // short circuit
    let r1 = side(1, false) && side(2, true);
    println!("r1 = {}", r1);
    let r2 = side(3, true) || side(4, false);
    println!("r2 = {}", r2);
    let r3 = side(5, true) && side(6, false);
    println!("r3 = {}", r3);
    let r4 = side(7, false) || side(8, true);
    println!("r4 = {}", r4);
    // non-short-circuit
    let r5 = side(9, false) & side(10, true);
    println!("r5 = {}", r5);
    let mut b = false;
    b |= true;
    println!("{}", b);
    b &= false;
    println!("{}", b);
    b ^= true;
    println!("{}", b);
    println!("{}", t == f);
    println!("{}", t != f);
    println!("{}", t > f);
    println!("{}", (3 < 4) == (5 < 6));
    let n = 10;
    let in_range = n >= 5 && n <= 15;
    println!("in_range = {}", in_range);
    println!("{}", t as u8 + t as u8);
}
