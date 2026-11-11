fn fact(n: u64) -> u64 {
    if n == 0 { 1 } else { n * fact(n - 1) }
}

fn fib(n: u32) -> u64 {
    if n < 2 { n as u64 } else { fib(n - 1) + fib(n - 2) }
}

fn ackermann(m: u64, n: u64) -> u64 {
    if m == 0 {
        n + 1
    } else if n == 0 {
        ackermann(m - 1, 1)
    } else {
        ackermann(m - 1, ackermann(m, n - 1))
    }
}

fn is_even(n: u32) -> bool {
    if n == 0 { true } else { is_odd(n - 1) }
}

fn is_odd(n: u32) -> bool {
    if n == 0 { false } else { is_even(n - 1) }
}

fn power(base: i64, exp: u32) -> i64 {
    if exp == 0 {
        1
    } else if exp % 2 == 0 {
        let h = power(base, exp / 2);
        h * h
    } else {
        base * power(base, exp - 1)
    }
}

fn hanoi(n: u32, from: u32, to: u32, via: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let a = hanoi(n - 1, from, via, to);
    if n >= 3 {
        println!("move disk {} from {} to {}", n, from, to);
    }
    let b = hanoi(n - 1, via, to, from);
    a + b + 1
}

fn main() {
    let mut i = 0;
    while i <= 20 {
        println!("fact({}) = {}", i, fact(i));
        i += 4;
    }
    let mut j = 0;
    while j <= 25 {
        println!("fib({}) = {}", j, fib(j));
        j += 5;
    }
    println!("ack(2,3) = {}", ackermann(2, 3));
    println!("ack(3,3) = {}", ackermann(3, 3));
    println!("even(10) = {} odd(10) = {}", is_even(10), is_odd(10));
    println!("even(7) = {} odd(7) = {}", is_even(7), is_odd(7));
    println!("3^13 = {}", power(3, 13));
    println!("-2^31 = {}", power(-2, 31));
    println!("hanoi moves = {}", hanoi(4, 1, 3, 2));
}
