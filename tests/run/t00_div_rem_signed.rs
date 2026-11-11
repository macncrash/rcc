fn show(a: i32, b: i32) {
    println!("{} / {} = {}, {} % {} = {}", a, b, a / b, a, b, a % b);
}

fn main() {
    show(7, 2);
    show(-7, 2);
    show(7, -2);
    show(-7, -2);
    show(0, 5);
    show(1, 1);
    show(i32::MAX, 2);
    show(i32::MIN, 2);
    show(i32::MIN, 3);
    show(-1, 1000);
    let a: i8 = -100;
    println!("{} {}", a / 7, a % 7);
    let b: i64 = -9_000_000_000_000;
    println!("{} {}", b / 7, b % 7);
    let c: u8 = 250;
    println!("{} {}", c / 7, c % 7);
    let d: u64 = u64::MAX;
    println!("{} {}", d / 10, d % 10);
    let e: i16 = -32768;
    println!("{} {}", e / 256, e % 256);
    let mut n: i32 = -100;
    while n <= 100 {
        println!("{}: /8={} %8={} >>3={}", n, n / 8, n % 8, n >> 3);
        n += 37;
    }
}
