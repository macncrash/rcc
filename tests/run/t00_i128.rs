fn main() {
    println!("{}", i128::MIN);
    println!("{}", i128::MAX);
    let a: i128 = -170141183460469231731687303715884105727;
    println!("{}", a - 1);
    let b: i128 = -12345678901234567890123456;
    let c: i128 = 987654321987;
    println!("{}", b / c);
    println!("{}", b % c);
    println!("{}", -b / c);
    println!("{}", b * 3);
    println!("{}", b >> 10);
    println!("{}", b << 5);
    println!("{}", b as u128);
    println!("{}", (b as u128) >> 100);
    let d: i64 = i64::MIN;
    let e = d as i128 * d as i128;
    println!("{}", e);
    let f = (d as i128) * (i64::MAX as i128);
    println!("{}", f);
    println!("{}", f as i64);
    let mut p: i128 = 1;
    let mut i = 0;
    while i < 40 {
        p *= -7;
        if i % 8 == 7 {
            println!("(-7)^{} = {}", i + 1, p);
        }
        i += 1;
    }
    println!("{}", i128::MAX.wrapping_add(1) == i128::MIN);
    println!("{}", (-1i128) as u8);
    println!("{}", -5i128 < 3i128);
}
