fn main() {
    let mut i = 0;
    let mut sum = 0;
    while i <= 20 {
        sum += i;
        i += 1;
    }
    println!("sum 0..=20 = {}", sum);
    let mut n: u64 = 1;
    let mut k = 0;
    while n < 1_000_000_000_000 {
        n *= 7;
        k += 1;
    }
    println!("7^{} = {}", k, n);
    let mut a = 0;
    while a < 5 {
        let mut b = 0;
        while b < a {
            println!("a={} b={}", a, b);
            b += 1;
        }
        a += 1;
    }
    let mut countdown = 10;
    while countdown > 0 {
        if countdown % 3 == 0 {
            countdown -= 1;
            continue;
        }
        println!("countdown {}", countdown);
        countdown -= 1;
    }
    let mut x = 100;
    while x > 1 {
        if x == 13 {
            break;
        }
        x -= 1;
    }
    println!("stopped at {}", x);
    let mut never = 0;
    while false {
        never += 1;
    }
    println!("never = {}", never);
}
