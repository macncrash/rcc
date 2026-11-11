// where clauses, multiple bounds, bounds on associated types.
use std::fmt::Display;
use std::ops::{Add, Mul};

fn dot<T>(a: &[T], b: &[T]) -> T
where
    T: Copy + Add<Output = T> + Mul<Output = T> + Default,
{
    let mut acc = T::default();
    for i in 0..a.len() {
        acc = acc + a[i] * b[i];
    }
    acc
}

fn show_both<A, B>(a: A, b: B) -> String
where
    A: Display,
    B: Display + Copy,
{
    format!("<{}|{}|{}>", a, b, b)
}

trait Source {
    type Out;
    fn produce(&self, i: u32) -> Self::Out;
}

struct Squares;
impl Source for Squares {
    type Out = u64;
    fn produce(&self, i: u32) -> u64 {
        (i as u64) * (i as u64)
    }
}

struct Labels;
impl Source for Labels {
    type Out = char;
    fn produce(&self, i: u32) -> char {
        (b'a' + (i as u8)) as char
    }
}

fn print_n<S>(s: &S, n: u32)
where
    S: Source,
    S::Out: Display,
{
    let mut i = 0;
    while i < n {
        print!("{} ", s.produce(i));
        i += 1;
    }
    println!();
}

struct Wrap<T>(T);
impl<T> Wrap<T>
where
    T: Display + PartialOrd,
{
    fn bigger(self, other: T) -> String {
        if self.0 > other { format!("{}", self.0) } else { format!("{}", other) }
    }
}

fn main() {
    println!("{}", dot(&[1, 2, 3], &[4, 5, 6]));
    println!("{}", dot(&[0.5, 1.5], &[2.0, 4.0]));
    println!("{}", dot::<u8>(&[10, 10], &[10, 15]));
    println!("{}", show_both(1, 'x'));
    println!("{}", show_both("str", 2.5));
    print_n(&Squares, 6);
    print_n(&Labels, 5);
    println!("{}", Wrap(3).bigger(9));
    println!("{}", Wrap("pear").bigger("apple"));
}
