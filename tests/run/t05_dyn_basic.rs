// &dyn Trait: dynamic dispatch through references, heterogeneous arrays of &dyn.
trait Shape {
    fn area(&self) -> f64;
    fn name(&self) -> String;
    fn scale(&self, k: f64) -> f64 {
        self.area() * k * k
    }
}

struct Sq(f64);
struct Rect(f64, f64);
struct Circle {
    r: f64,
}

impl Shape for Sq {
    fn area(&self) -> f64 {
        self.0 * self.0
    }
    fn name(&self) -> String {
        format!("square({})", self.0)
    }
}
impl Shape for Rect {
    fn area(&self) -> f64 {
        self.0 * self.1
    }
    fn name(&self) -> String {
        format!("rect({}x{})", self.0, self.1)
    }
    fn scale(&self, k: f64) -> f64 {
        // override the default
        -self.area() * k
    }
}
impl Shape for Circle {
    fn area(&self) -> f64 {
        3.25 * self.r * self.r
    }
    fn name(&self) -> String {
        String::from("circle")
    }
}

fn describe(s: &dyn Shape) {
    println!("{} area={} scaled={}", s.name(), s.area(), s.scale(2.0));
}

fn biggest<'a>(shapes: &[&'a dyn Shape]) -> &'a dyn Shape {
    let mut best = shapes[0];
    for s in shapes {
        if s.area() > best.area() {
            best = *s;
        }
    }
    best
}

impl Shape for i32 {
    fn area(&self) -> f64 {
        *self as f64
    }
    fn name(&self) -> String {
        format!("int {}", self)
    }
}

fn main() {
    let a = Sq(3.0);
    let b = Rect(2.0, 5.5);
    let c = Circle { r: 2.0 };
    describe(&a);
    describe(&b);
    describe(&c);
    describe(&7);
    let all: [&dyn Shape; 4] = [&a, &b, &c, &20];
    let mut total = 0.0;
    for s in all.iter() {
        total += s.area();
    }
    println!("total {}", total);
    println!("biggest {}", biggest(&all).name());
    println!("biggest {}", biggest(&all[..3]).name());
    let r: &dyn Shape = if total > 50.0 { &a } else { &b };
    println!("{}", r.name());
    println!("{}", std::mem::size_of::<&dyn Shape>() == 2 * std::mem::size_of::<usize>());
}
