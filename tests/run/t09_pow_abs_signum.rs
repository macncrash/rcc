// pow, abs, signum, unsigned_abs, abs_diff, is_positive/is_negative, isqrt-like loops
fn main() {
    for base in [0i64, 1, -1, 2, -3, 10] {
        for e in [0u32, 1, 2, 5, 13] {
            print!("{}^{}={} ", base, e, base.pow(e));
        }
        println!();
    }
    println!("2^63 u64 {}", 2u64.pow(63));
    println!("3^80 u128 {}", 3u128.pow(80));
    println!("(-2)^127 i128 {}", (-2i128).pow(127));
    for x in [i32::MIN + 1, -100, -1, 0, 1, 100, i32::MAX] {
        println!("i32 {}: abs {} signum {} pos {} neg {} uabs {}", x, x.abs(), x.signum(), x.is_positive(), x.is_negative(), x.unsigned_abs());
    }
    println!("i32::MIN unsigned_abs {}", i32::MIN.unsigned_abs());
    println!("i8::MIN unsigned_abs {}", i8::MIN.unsigned_abs());
    println!("i128::MIN unsigned_abs {}", i128::MIN.unsigned_abs());
    for (a, b) in [(3u32, 10u32), (10, 3), (0, u32::MAX)] {
        println!("abs_diff u32 {} {} = {}", a, b, a.abs_diff(b));
    }
    for (a, b) in [(-5i32, 7i32), (i32::MIN, i32::MAX), (100, -100)] {
        println!("abs_diff i32 {} {} = {}", a, b, a.abs_diff(b));
    }
    // integer square root by Newton
    for n in [0u64, 1, 2, 15, 16, 17, 1_000_000, 999_999_999_999, u64::MAX] {
        let r = if n < 2 {
            n
        } else {
            let mut x = n;
            let mut y = x / 2 + (x & 1);
            while y < x {
                x = y;
                y = (x + n / x) / 2;
            }
            x
        };
        println!("isqrt {} = {}", n, r);
    }
    // modular exponentiation with u128 intermediates
    let m: u64 = 1_000_000_007;
    let mut result: u64 = 1;
    let mut b: u64 = 123456789;
    let mut e: u64 = 1_000_000;
    while e > 0 {
        if e & 1 == 1 {
            result = ((result as u128 * b as u128) % m as u128) as u64;
        }
        b = ((b as u128 * b as u128) % m as u128) as u64;
        e >>= 1;
    }
    println!("modpow {}", result);
    let fs: [f64; 4] = [-2.5, -0.0, 0.0, 3.0];
    for &f in fs.iter() {
        println!("f64 {} abs {} signum {}", f, f.abs(), f.signum());
    }
}
