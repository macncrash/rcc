fn main() {
    let a = 17;
    let b = 5;
    println!("{}", a + b);
    println!("{}", a - b);
    println!("{}", a * b);
    println!("{}", a / b);
    println!("{}", a % b);
    println!("{}", b - a);
    println!("{}", -a / b);
    println!("{}", -a % b);
    println!("{}", a / -b);
    println!("{}", a % -b);
    println!("{}", (a + b) * (a - b));
    println!("{}", a + b * 2 - 3);
    println!("{}", (a + b) * 2 - 3);
    println!("{}", 100 - 10 - 1);
    println!("{}", 1000 / 10 / 3);
    println!("{}", 2 * 3 + 4 * 5);
    println!("{}", 7 - -3);
    let big: i64 = 1_000_000_007;
    println!("{}", big * big);
    println!("{}", big * big % 998_244_353);
    let mut acc: i32 = 1;
    let mut i = 0;
    while i < 10 {
        acc = acc * 3 + i;
        println!("acc {} = {}", i, acc);
        i += 1;
    }
    let x: u32 = 4_000_000_000;
    println!("{}", x / 7);
    println!("{}", x % 7);
    println!("{}", 0x1F + 0o17 + 0b1010);
}
