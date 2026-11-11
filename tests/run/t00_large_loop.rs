// Exercises tight loops: output only depends on arithmetic, keep it fast.
fn main() {
    let mut sum: u64 = 0;
    let mut i: u64 = 0;
    while i < 10_000_000 {
        sum += i ^ (i >> 3);
        i += 1;
    }
    println!("sum = {}", sum);
    let mut x: u32 = 12345;
    let mut j = 0;
    let mut hist0 = 0;
    let mut hist1 = 0;
    while j < 1_000_000 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        if x & 1 == 0 { hist0 += 1 } else { hist1 += 1 }
        j += 1;
    }
    println!("xorshift final = {} evens = {} odds = {}", x, hist0, hist1);
    let mut sieve_count = 0;
    let mut n = 2;
    while n < 30000 {
        let mut d = 2;
        let mut prime = true;
        while d * d <= n {
            if n % d == 0 {
                prime = false;
                break;
            }
            d += 1;
        }
        if prime {
            sieve_count += 1;
        }
        n += 1;
    }
    println!("primes < 30000 = {}", sieve_count);
}
