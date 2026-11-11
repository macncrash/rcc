// Trait associated functions without self (constructors), Self in traits,
// calling via T::f() in generic code.
trait Zero {
    fn zero() -> Self;
    fn is_zero(&self) -> bool;
}

trait Monoid: Zero + Copy {
    fn combine(a: Self, b: Self) -> Self;
    fn concat(xs: &[Self]) -> Self {
        let mut acc = Self::zero();
        for x in xs {
            acc = Self::combine(acc, *x);
        }
        acc
    }
}

impl Zero for i32 {
    fn zero() -> i32 {
        0
    }
    fn is_zero(&self) -> bool {
        *self == 0
    }
}
impl Monoid for i32 {
    fn combine(a: i32, b: i32) -> i32 {
        a + b
    }
}

#[derive(Clone, Copy, Debug)]
struct MaxU(u32);
impl Zero for MaxU {
    fn zero() -> Self {
        MaxU(0)
    }
    fn is_zero(&self) -> bool {
        self.0 == 0
    }
}
impl Monoid for MaxU {
    fn combine(a: Self, b: Self) -> Self {
        if a.0 > b.0 { a } else { b }
    }
}

#[derive(Clone, Copy, Debug)]
struct Mat2([i64; 4]);
impl Zero for Mat2 {
    // identity for multiplication
    fn zero() -> Self {
        Mat2([1, 0, 0, 1])
    }
    fn is_zero(&self) -> bool {
        self.0 == [1, 0, 0, 1]
    }
}
impl Monoid for Mat2 {
    fn combine(a: Self, b: Self) -> Self {
        let a = a.0;
        let b = b.0;
        Mat2([
            a[0] * b[0] + a[1] * b[2],
            a[0] * b[1] + a[1] * b[3],
            a[2] * b[0] + a[3] * b[2],
            a[2] * b[1] + a[3] * b[3],
        ])
    }
}

fn power<M: Monoid>(x: M, mut n: u32) -> M {
    let mut result = M::zero();
    let mut base = x;
    while n > 0 {
        if n & 1 == 1 {
            result = M::combine(result, base);
        }
        base = M::combine(base, base);
        n >>= 1;
    }
    result
}

trait Named {
    fn create(name: &'static str) -> Self;
    fn label(&self) -> String;
}
struct Tag(&'static str);
impl Named for Tag {
    fn create(name: &'static str) -> Self {
        Tag(name)
    }
    fn label(&self) -> String {
        format!("#{}", self.0)
    }
}
fn build<T: Named>() -> T {
    T::create("built")
}

fn main() {
    println!("{}", i32::concat(&[1, 2, 3, 4]));
    println!("{}", <i32 as Zero>::zero().is_zero());
    println!("{:?}", MaxU::concat(&[MaxU(3), MaxU(17), MaxU(5)]));
    println!("{}", MaxU::concat(&[]).is_zero());
    println!("{}", power(3i32, 5));
    let fib = power(Mat2([1, 1, 1, 0]), 50);
    println!("{:?}", fib);
    println!("{}", power(Mat2([2, 0, 0, 2]), 0).is_zero());
    let t: Tag = build();
    println!("{}", t.label());
    println!("{}", Tag::create("direct").label());
}
