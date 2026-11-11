fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

fn isqrt(n: u64) -> u64 {
    let mut lo = 0u64;
    let mut hi = 1u64 << 32;
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        if mid * mid <= n { lo = mid } else { hi = mid - 1 }
    }
    lo
}

fn modpow(mut base: u64, mut exp: u64, m: u64) -> u64 {
    let mut result = 1;
    base %= m;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result * base % m;
        }
        base = base * base % m;
        exp >>= 1;
    }
    result
}

fn main() {
    println!("gcd(48,18) = {}", gcd(48, 18));
    println!("gcd(1071,462) = {}", gcd(1071, 462));
    println!("gcd(2^40,6^20) = {}", gcd(1 << 40, 3656158440062976));
    println!("lcm(21,6) = {}", 21 * 6 / gcd(21, 6));
    let mut count = 0;
    let mut n = 0;
    while n < 200 {
        if is_prime(n) {
            count += 1;
            if count % 5 == 0 {
                println!("prime #{} = {}", count, n);
            }
        }
        n += 1;
    }
    println!("primes below 200: {}", count);
    println!("is_prime(1000003) = {}", is_prime(1000003));
    println!("is_prime(1000001) = {}", is_prime(1000001));
    println!("isqrt(99) = {}", isqrt(99));
    println!("isqrt(100) = {}", isqrt(100));
    println!("isqrt(10^18) = {}", isqrt(1_000_000_000_000_000_000));
    println!("isqrt(2^63) = {}", isqrt(1 << 63));
    println!("3^200 mod 1e9+7 = {}", modpow(3, 200, 1_000_000_007));
    println!("2^(p-1) mod p = {}", modpow(2, 998244352, 998244353));
    let mut perfect = 1;
    while perfect < 10000 {
        let mut s = 0;
        let mut d = 1;
        while d < perfect {
            if perfect % d == 0 {
                s += d;
            }
            d += 1;
        }
        if s == perfect {
            println!("perfect {}", perfect);
        }
        perfect += 1;
    }
}
