fn main() {
    let a: u128 = 1 << 100;
    println!("{}", a);
    println!("{}", u128::MAX);
    println!("{}", u128::MAX / 3);
    println!("{}", u128::MAX % 1_000_000_007);
    let b: u128 = 0xFEDC_BA98_7654_3210_0123_4567_89AB_CDEF;
    println!("{}", b);
    println!("{}", b >> 64);
    println!("{}", b as u64);
    println!("{}", b & 0xFFFF_FFFF);
    let x: u64 = 0xFFFF_FFFF_FFFF_FFFF;
    let wide = x as u128 * x as u128;
    println!("{}", wide);
    println!("{}", (wide >> 64) as u64);
    println!("{}", wide as u64);
    let mut f: u128 = 1;
    let mut i: u128 = 1;
    while i <= 34 {
        f *= i;
        if i % 6 == 4 {
            println!("{}! = {}", i, f);
        }
        i += 1;
    }
    println!("{}", f / 1_000_000_000_000_000_000_000);
    println!("{}", f % 1_000_000_000_000_000_000_000);
    let c: u128 = 12345678901234567890123456789;
    let d: u128 = 98765432109876543210;
    println!("{}", c / d);
    println!("{}", c % d);
    println!("{}", c + d);
    println!("{}", c - d);
    println!("{}", c ^ d);
    println!("{}", c > d);
    println!("{}", 0u128.wrapping_sub(1) == u128::MAX);
    println!("{}", (1u128 << 127) >> 127);
}
