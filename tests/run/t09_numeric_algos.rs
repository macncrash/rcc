// small numeric kernels that exercise mixed-width arithmetic
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2u32;
    while (d as u64) * (d as u64) <= n as u64 {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn fixed_mul(a: i32, b: i32) -> i32 {
    // Q16.16 fixed point
    ((a as i64 * b as i64) >> 16) as i32
}

fn main() {
    println!("gcd {} {} {}", gcd(48, 18), gcd(1071, 462), gcd(u64::MAX, 3));
    let mut primes = Vec::new();
    for n in 0..100 {
        if is_prime(n) {
            primes.push(n);
        }
    }
    println!("primes {:?}", primes);
    println!("is_prime(4294967291) {}", is_prime(4294967291));
    println!("crc32 {:08x}", crc32(b"The quick brown fox jumps over the lazy dog"));
    println!("crc32 empty {:08x}", crc32(b""));
    let mut st: u64 = 88172645463325252;
    for _ in 0..8 {
        println!("xorshift {}", xorshift(&mut st));
    }
    let one = 1 << 16;
    let half = one / 2;
    let three = 3 * one;
    println!("fixed {} {} {}", fixed_mul(half, three), fixed_mul(-three, three), fixed_mul(one + half, one + half));
    // collatz lengths
    let mut best = (0u64, 0u32);
    for start in 1u64..3000 {
        let mut n = start;
        let mut len = 0u32;
        while n != 1 {
            n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
            len += 1;
        }
        if len > best.1 {
            best = (start, len);
        }
    }
    println!("collatz best {:?}", best);
    // fibonacci mod 2^64 and exact in u128
    let (mut a, mut b) = (0u128, 1u128);
    for i in 0..=180 {
        if i % 30 == 0 {
            println!("fib {} = {} (low64 {})", i, a, a as u64);
        }
        let t = a + b;
        a = b;
        b = t;
    }
    // integer average without overflow
    let pairs: [(u32, u32); 3] = [(u32::MAX, u32::MAX - 2), (1, 2), (0, u32::MAX)];
    for &(x, y) in pairs.iter() {
        println!("avg {} {} = {}", x, y, (x & y) + ((x ^ y) >> 1));
    }
}
