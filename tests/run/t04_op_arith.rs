// Operator overloading: Add, Sub, Mul, Div, Rem, Neg, including mixed RHS types
// and impls on references.
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

#[derive(Clone, Copy, Debug, PartialEq)]
struct V2 {
    x: i32,
    y: i32,
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        V2 { x: self.x + o.x, y: self.y + o.y }
    }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        V2 { x: self.x - o.x, y: self.y - o.y }
    }
}
impl Mul<i32> for V2 {
    type Output = V2;
    fn mul(self, k: i32) -> V2 {
        V2 { x: self.x * k, y: self.y * k }
    }
}
impl Mul<V2> for i32 {
    type Output = V2;
    fn mul(self, v: V2) -> V2 {
        v * self
    }
}
// dot product
impl Mul for V2 {
    type Output = i32;
    fn mul(self, o: V2) -> i32 {
        self.x * o.x + self.y * o.y
    }
}
impl Div<i32> for V2 {
    type Output = V2;
    fn div(self, k: i32) -> V2 {
        V2 { x: self.x / k, y: self.y / k }
    }
}
impl Rem<i32> for V2 {
    type Output = V2;
    fn rem(self, k: i32) -> V2 {
        V2 { x: self.x % k, y: self.y % k }
    }
}
impl Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        V2 { x: -self.x, y: -self.y }
    }
}
impl<'a> Add<&'a V2> for &'a V2 {
    type Output = V2;
    fn add(self, o: &V2) -> V2 {
        *self + *o
    }
}

// A non-Copy type with operators
#[derive(Debug, Clone, PartialEq)]
struct Poly(Vec<i64>);

impl Add for Poly {
    type Output = Poly;
    fn add(self, o: Poly) -> Poly {
        let n = if self.0.len() > o.0.len() { self.0.len() } else { o.0.len() };
        let mut r = Vec::new();
        for i in 0..n {
            let a = if i < self.0.len() { self.0[i] } else { 0 };
            let b = if i < o.0.len() { o.0[i] } else { 0 };
            r.push(a + b);
        }
        Poly(r)
    }
}
impl Mul for &Poly {
    type Output = Poly;
    fn mul(self, o: &Poly) -> Poly {
        let mut r = vec![0; self.0.len() + o.0.len() - 1];
        for i in 0..self.0.len() {
            for j in 0..o.0.len() {
                r[i + j] += self.0[i] * o.0[j];
            }
        }
        Poly(r)
    }
}

fn sum_generic<T: Add<Output = T> + Copy>(a: T, b: T, c: T) -> T {
    a + b + c
}

fn main() {
    let a = V2 { x: 1, y: 2 };
    let b = V2 { x: 10, y: -20 };
    println!("{:?}", a + b);
    println!("{:?}", a - b);
    println!("{:?}", a * 3);
    println!("{:?}", 4 * b);
    println!("{}", a * b);
    println!("{:?}", b / 3);
    println!("{:?}", b % 3);
    println!("{:?}", -a);
    println!("{:?}", -(a + b) * 2);
    println!("{:?}", &a + &b);
    println!("{:?}", sum_generic(a, b, a));
    println!("{}", a + b == V2 { x: 11, y: -18 });
    let p = Poly(vec![1, 1]);
    let q = &p * &p;
    println!("{:?}", q);
    let r = &q * &p;
    println!("{:?}", r);
    println!("{:?}", r.clone() + Poly(vec![5]));
    println!("{:?}", Poly(vec![1]) + r);
}
