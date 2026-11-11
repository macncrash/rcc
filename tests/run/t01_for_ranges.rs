fn main() {
    for i in 0..5 {
        print!("{} ", i);
    }
    println!();
    for i in 0..=5 {
        print!("{} ", i);
    }
    println!();
    for i in -3..3 {
        print!("{} ", i);
    }
    println!();
    for i in (0..5).rev() {
        print!("{} ", i);
    }
    println!();
    for i in (1..=4).rev() {
        print!("{} ", i);
    }
    println!();
    for _ in 0..3 {
        print!("x");
    }
    println!();
    for i in 5..5 {
        println!("never {}", i);
    }
    for i in 5..2 {
        println!("never {}", i);
    }
    let lo = 10u8;
    let hi = 15u8;
    for b in lo..hi {
        print!("{} ", b);
    }
    println!();
    for b in 250u8..=255 {
        print!("{} ", b);
    }
    println!();
    for c in 'a'..='e' {
        print!("{}", c);
    }
    println!();
    let mut total: i64 = 0;
    for i in 1..=100i64 {
        total += i * i;
    }
    println!("sum of squares = {}", total);
    let n = 10;
    let mut prod: u64 = 1;
    for k in 1..n + 1 {
        prod *= k;
    }
    println!("10! = {}", prod);
    for i in 0..20 {
        if i % 3 != 0 {
            continue;
        }
        if i > 15 {
            break;
        }
        print!("{} ", i);
    }
    println!();
    let r = 3..7;
    println!("{} {}", r.start, r.end);
    let mut sum = 0;
    for x in r {
        sum += x;
    }
    println!("sum 3..7 = {}", sum);
}
