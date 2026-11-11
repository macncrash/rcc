// Blanket impls: impl<T: Bound> Trait for T, and impls for references.
use std::fmt::Display;

trait Describe {
    fn describe(&self) -> String;
}

impl<T: Display> Describe for T {
    fn describe(&self) -> String {
        format!("<<{}>>", self)
    }
}

trait Double {
    fn double(&self) -> Self;
}
impl<T: Copy + std::ops::Add<Output = T>> Double for T {
    fn double(&self) -> T {
        *self + *self
    }
}

trait Len {
    fn length(&self) -> usize;
}
impl Len for str {
    fn length(&self) -> usize {
        self.len()
    }
}
impl<T> Len for [T] {
    fn length(&self) -> usize {
        self.len()
    }
}
impl<T: Len + ?Sized> Len for &T {
    fn length(&self) -> usize {
        (**self).length() * 100
    }
}

fn total_len<A: Len, B: Len>(a: A, b: B) -> usize {
    a.length() + b.length()
}

trait Shout {
    fn shout(&self) -> String;
}
struct Name(&'static str);
impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Name={}", self.0)
    }
}
impl<T: Describe> Shout for Vec<T> {
    fn shout(&self) -> String {
        let mut s = String::new();
        for x in self {
            s.push_str(&x.describe());
        }
        s
    }
}

fn main() {
    println!("{}", 5.describe());
    println!("{}", "str".describe());
    println!("{}", 'c'.describe());
    println!("{}", Name("bob").describe());
    println!("{}", 21.double());
    println!("{}", 1.25.double());
    println!("{}", 200u8.double());
    println!("{}", "hello".length());
    println!("{}", [1, 2, 3][..].length());
    println!("{}", total_len("abc", &[0u8; 4][..]));
    println!("{}", vec![1, 2].shout());
    println!("{}", vec![Name("x"), Name("y")].shout());
}
