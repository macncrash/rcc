fn cmp(a: i64, b: i64) {
    println!("{} {}: < {} <= {} > {} >= {} == {} != {}", a, b, a < b, a <= b, a > b, a >= b, a == b, a != b);
}

fn cmpu(a: u64, b: u64) {
    println!("{} {}: < {} > {} == {}", a, b, a < b, a > b, a == b);
}

fn main() {
    cmp(1, 2);
    cmp(2, 1);
    cmp(3, 3);
    cmp(-5, 5);
    cmp(i64::MIN, i64::MAX);
    cmp(-1, 0);
    cmpu(u64::MAX, 0);
    cmpu(1 << 63, (1 << 63) - 1);
    let x: i8 = -1;
    let y: u8 = 255;
    println!("{}", x as u8 == y);
    println!("{}", (y as i8) < 0);
    println!("{}", 'a' < 'b');
    println!("{}", 'Z' < 'a');
    println!("{}", 1.5 < 2.5);
    println!("{}", -0.0 == 0.0);
    let i: i32 = -10;
    let u: u32 = i as u32;
    println!("{} {}", i < 0, u > 1000);
    println!("{}", 100u8 > 99u8);
    println!("{}", -128i8 < 127i8);
    println!("{}", (1u128 << 100) > (1u128 << 99));
    println!("{}", (-1i128) < 0);
}
