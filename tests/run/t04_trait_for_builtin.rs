// Implementing user traits for primitives, tuples, arrays, slices, Option.
trait Weight {
    fn weight(&self) -> u64;
}

impl Weight for u8 {
    fn weight(&self) -> u64 {
        *self as u64
    }
}
impl Weight for i32 {
    fn weight(&self) -> u64 {
        self.unsigned_abs() as u64
    }
}
impl Weight for bool {
    fn weight(&self) -> u64 {
        if *self { 1 } else { 0 }
    }
}
impl Weight for char {
    fn weight(&self) -> u64 {
        *self as u64
    }
}
impl Weight for &str {
    fn weight(&self) -> u64 {
        let mut s = 0;
        for c in self.chars() {
            s += c.weight();
        }
        s
    }
}
impl<A: Weight, B: Weight> Weight for (A, B) {
    fn weight(&self) -> u64 {
        self.0.weight() * 1000 + self.1.weight()
    }
}
impl<T: Weight, const N: usize> Weight for [T; N] {
    fn weight(&self) -> u64 {
        let mut s = 0;
        for x in self.iter() {
            s += x.weight();
        }
        s + N as u64
    }
}
impl<T: Weight> Weight for Option<T> {
    fn weight(&self) -> u64 {
        match self {
            Some(x) => x.weight() + 1,
            None => 0,
        }
    }
}
impl Weight for () {
    fn weight(&self) -> u64 {
        42
    }
}

trait Ext {
    fn is_even_ext(self) -> bool;
    fn digits(self) -> u32;
}
impl Ext for u64 {
    fn is_even_ext(self) -> bool {
        self % 2 == 0
    }
    fn digits(self) -> u32 {
        if self < 10 { 1 } else { 1 + (self / 10).digits() }
    }
}

fn w<T: Weight>(t: T) -> u64 {
    t.weight()
}

fn main() {
    println!("{}", w(200u8));
    println!("{}", w(-77i32));
    println!("{}", w(true));
    println!("{}", w('A'));
    println!("{}", w("AB"));
    println!("{}", w((3u8, false)));
    println!("{}", w([1u8, 2, 3]));
    println!("{}", w([(1u8, 1u8), (2, 2)]));
    println!("{}", w(Some(5i32)));
    println!("{}", w(None::<char>));
    println!("{}", w(Some(Some(true))));
    println!("{}", w(()));
    println!("{} {}", 10u64.is_even_ext(), 7u64.is_even_ext());
    println!("{} {} {}", 0u64.digits(), 12345u64.digits(), u64::MAX.digits());
}
