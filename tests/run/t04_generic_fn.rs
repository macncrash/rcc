// Generic functions with trait bounds from std (Copy, PartialOrd, Add).
use std::ops::Add;

fn identity<T>(x: T) -> T {
    x
}

fn largest<T: PartialOrd + Copy>(xs: &[T]) -> T {
    let mut m = xs[0];
    let mut i = 1;
    while i < xs.len() {
        if xs[i] > m {
            m = xs[i];
        }
        i += 1;
    }
    m
}

fn sum_all<T: Add<Output = T> + Copy>(xs: &[T], zero: T) -> T {
    let mut acc = zero;
    for x in xs {
        acc = acc + *x;
    }
    acc
}

fn swap_pair<A, B>(p: (A, B)) -> (B, A) {
    (p.1, p.0)
}

fn apply_twice<T: Copy>(x: T, f: fn(T) -> T) -> T {
    f(f(x))
}

fn double_i32(x: i32) -> i32 {
    x * 2
}
fn half_f64(x: f64) -> f64 {
    x / 2.0
}

fn count_eq<T: PartialEq>(xs: &[T], v: &T) -> usize {
    let mut n = 0;
    for x in xs {
        if x == v {
            n += 1;
        }
    }
    n
}

fn main() {
    println!("{}", identity(5));
    println!("{}", identity(true));
    println!("{}", identity('z'));
    println!("{}", identity(2.5f64));
    println!("{}", largest(&[3, 9, 2, 7]));
    println!("{}", largest(&[1.5, -2.0, 0.25]));
    println!("{}", largest(&['a', 'q', 'c']));
    println!("{}", largest(&[200u8, 255, 3]));
    println!("{}", sum_all(&[1, 2, 3, 4, 5], 0));
    println!("{}", sum_all(&[1.5, 2.25], 0.0));
    println!("{}", sum_all(&[10u64, 20, 30], 0u64));
    let (a, b) = swap_pair((1u8, 'x'));
    println!("{} {}", a, b);
    let (c, d) = swap_pair((true, -7i64));
    println!("{} {}", c, d);
    println!("{}", apply_twice(5, double_i32));
    println!("{}", apply_twice(10.0, half_f64));
    println!("{}", count_eq(&[1, 2, 1, 3, 1], &1));
    println!("{}", count_eq(&['a', 'b', 'b'], &'b'));
    // turbofish
    println!("{}", identity::<u16>(65535));
    println!("{}", largest::<i8>(&[-5, -3, -100]));
}
