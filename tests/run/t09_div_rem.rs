// signed/unsigned division and remainder on every width, by runtime divisors
fn main() {
    let nums: [i64; 6] = [-100, -7, -1, 0, 7, 100];
    let dens: [i64; 4] = [-7, -3, 3, 7];
    for &n in nums.iter() {
        for &d in dens.iter() {
            print!("{}/{}={},{} ", n, d, n / d, n % d);
        }
        println!();
    }
    for &n in [-128i8, -77, 77, 127].iter() {
        for &d in [-3i8, 5].iter() {
            println!("i8 {}/{} = {} r {}", n, d, n / d, n % d);
        }
    }
    for &n in [0u8, 77, 200, 255].iter() {
        for &d in [1u8, 3, 16, 255].iter() {
            print!("{}/{}={},{} ", n, d, n / d, n % d);
        }
        println!();
    }
    for &n in [u16::MAX, 12345].iter() {
        println!("u16 {} /10 {} %10 {} /256 {} %256 {}", n, n / 10, n % 10, n / 256, n % 256);
    }
    for &n in [i32::MIN, -1_000_000_007, 999_999_937].iter() {
        println!("i32 {} /1000 {} %1000 {} /-1000 {} %-1000 {}", n, n / 1000, n % 1000, n / -1000, n % -1000);
    }
    let big: u64 = 0xFFFF_FFFF_FFFF_FFFF;
    for &d in [1u64, 2, 3, 10, 0x1_0000_0001, big].iter() {
        println!("u64 max / {} = {} r {}", d, big / d, big % d);
    }
    // usize division used in indexing
    let data = [10, 20, 30, 40, 50, 60, 70];
    let mut idx: usize = 123456;
    for _ in 0..5 {
        println!("idx {} -> data[{}] = {}", idx, idx % data.len(), data[idx % data.len()]);
        idx = idx / 3 + 17;
    }
    // division by powers of two must round toward zero for signed values
    for &n in [-9i32, -8, -1, 1, 9].iter() {
        println!("{} /2 {} /4 {} >>1 {} >>2 {}", n, n / 2, n / 4, n >> 1, n >> 2);
    }
    // float division and remainder
    for &(a, b) in [(7.5f64, 2.0f64), (-7.5, 2.0), (1.0, 0.0), (0.0, -1.0)].iter() {
        println!("f64 {}/{} = {} rem {}", a, b, a / b, a % b);
    }
}
