// Trait definition, impls on several types, static dispatch through generics.
trait Shape {
    fn area(&self) -> f64;
    fn name(&self) -> &'static str;
    fn sides(&self) -> u32;
}

struct Circle {
    r: f64,
}
struct Rect {
    w: f64,
    h: f64,
}
struct Tri {
    a: f64,
    b: f64,
    c: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.0 * self.r * self.r
    }
    fn name(&self) -> &'static str {
        "circle"
    }
    fn sides(&self) -> u32 {
        0
    }
}

impl Shape for Rect {
    fn area(&self) -> f64 {
        self.w * self.h
    }
    fn name(&self) -> &'static str {
        "rect"
    }
    fn sides(&self) -> u32 {
        4
    }
}

impl Shape for Tri {
    fn area(&self) -> f64 {
        let s = (self.a + self.b + self.c) / 2.0;
        (s * (s - self.a) * (s - self.b) * (s - self.c)).sqrt()
    }
    fn name(&self) -> &'static str {
        "tri"
    }
    fn sides(&self) -> u32 {
        3
    }
}

fn describe<S: Shape>(s: &S) {
    println!("{} sides={} area={}", s.name(), s.sides(), s.area());
}

fn total_area<A: Shape, B: Shape>(a: &A, b: &B) -> f64 {
    a.area() + b.area()
}

trait Counter {
    fn incr(&mut self);
    fn get(&self) -> i32;
}

struct ByOne(i32);
struct ByTen(i32);

impl Counter for ByOne {
    fn incr(&mut self) {
        self.0 += 1;
    }
    fn get(&self) -> i32 {
        self.0
    }
}
impl Counter for ByTen {
    fn incr(&mut self) {
        self.0 += 10;
    }
    fn get(&self) -> i32 {
        self.0
    }
}

fn run<C: Counter>(c: &mut C, n: u32) -> i32 {
    for _ in 0..n {
        c.incr();
    }
    c.get()
}

fn main() {
    let c = Circle { r: 2.0 };
    let r = Rect { w: 3.0, h: 4.5 };
    let t = Tri { a: 3.0, b: 4.0, c: 5.0 };
    describe(&c);
    describe(&r);
    describe(&t);
    println!("{}", total_area(&c, &r));
    println!("{}", total_area(&t, &t));
    // method call syntax and UFCS
    println!("{}", Shape::area(&r));
    println!("{}", <Tri as Shape>::name(&t));
    let mut a = ByOne(0);
    let mut b = ByTen(5);
    println!("{}", run(&mut a, 3));
    println!("{}", run(&mut b, 3));
    println!("{}", run(&mut a, 4));
}
