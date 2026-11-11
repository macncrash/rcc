fn reverse(mut n: u64) -> u64 {
    let mut r = 0;
    while n > 0 {
        r = r * 10 + n % 10;
        n /= 10;
    }
    r
}

fn digit_sum(mut n: u64) -> u64 {
    let mut s = 0;
    while n > 0 {
        s += n % 10;
        n /= 10;
    }
    s
}

fn count_digits(mut n: u64, base: u64) -> u32 {
    if n == 0 {
        return 1;
    }
    let mut c = 0;
    while n > 0 {
        c += 1;
        n /= base;
    }
    c
}

fn print_binary(n: u32) {
    let mut bit = 31;
    let mut started = false;
    loop {
        let b = (n >> bit) & 1;
        if b == 1 {
            started = true;
        }
        if started {
            if b == 1 { print!("1") } else { print!("0") }
        }
        if bit == 0 {
            break;
        }
        bit -= 1;
    }
    if !started {
        print!("0");
    }
    println!();
}

fn main() {
    println!("{}", reverse(1234567));
    println!("{}", reverse(1000));
    println!("{}", reverse(18446744073709551));
    println!("{}", digit_sum(999999999));
    println!("{}", digit_sum(18446744073709551615));
    println!("{}", count_digits(0, 10));
    println!("{}", count_digits(u64::MAX, 10));
    println!("{}", count_digits(u64::MAX, 2));
    println!("{}", count_digits(255, 16));
    let mut n = 100;
    while n < 200 {
        if reverse(n) == n {
            print!("{} ", n);
        }
        n += 1;
    }
    println!();
    print_binary(0);
    print_binary(1);
    print_binary(10);
    print_binary(255);
    print_binary(0xDEADBEEF);
    let mut x: u64 = 1;
    let mut i = 0;
    while i < 15 {
        x *= 3;
        print!("{}:{} ", x, digit_sum(x));
        i += 1;
    }
    println!();
}
