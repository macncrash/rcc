// u128 arithmetic: full-width mul via u128, division by large divisors, shifts
fn mulhi(a: u64, b: u64) -> u64 {
    ((a as u128 * b as u128) >> 64) as u64
}

fn main() {
    let m = u128::MAX;
    println!("max {}", m);
    println!("max/3 {} max%3 {}", m / 3, m % 3);
    println!("max/(2^64+1) {}", m / ((1u128 << 64) + 1));
    let d: u128 = 0xFFFF_FFFF_FFFF_FFFF_0000_0001;
    println!("max/d {} max%d {}", m / d, m % d);
    println!("10^38 {}", 10u128.pow(38));
    let mut x: u128 = 1;
    for i in 0..40 {
        x = x.wrapping_mul(0x9E37_79B9_7F4A_7C15_F39C_C060_5CED_C835).wrapping_add(i);
        if i % 5 == 0 {
            println!("lcg {} {:x}", i, x);
        }
    }
    for s in [0u32, 1, 31, 32, 63, 64, 96, 127] {
        println!("s {}: {:x} {:x}", s, m >> s, 1u128 << s);
    }
    let pairs: [(u64, u64); 4] = [(u64::MAX, u64::MAX), (1 << 63, 2), (0x1234_5678_9abc_def0, 0xfedc_ba98_7654_3210), (3, 5)];
    for &(a, b) in pairs.iter() {
        println!("mulhi {:x} {:x} = {:x} lo {:x}", a, b, mulhi(a, b), a.wrapping_mul(b));
    }
    // decimal conversion by repeated division
    let mut v: u128 = 340282366920938463463374607431768211455;
    let mut digits = [0u8; 40];
    let mut n = 0;
    while v > 0 {
        digits[n] = (v % 10) as u8;
        v /= 10;
        n += 1;
    }
    let mut s = String::new();
    while n > 0 {
        n -= 1;
        s.push((b'0' + digits[n]) as char);
    }
    println!("manual {}", s);
    let a: u128 = 0xDEAD_BEEF_CAFE_BABE_0123_4567_89AB_CDEF;
    println!("and {:x} or {:x} xor {:x} not {:x}", a & 0xFFFF_0000, a | 1, a ^ m, !a);
    println!("cmp {} {} {}", a > (1 << 127), a < m, (a as u64) as u128 == a & 0xFFFF_FFFF_FFFF_FFFF);
    println!("parse {}", "123456789012345678901234567890".parse::<u128>().unwrap() * 3);
}
