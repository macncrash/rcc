// Inherent impls for specific instantiations and bounded impls.
struct Vec2<T> {
    x: T,
    y: T,
}

impl<T: Copy> Vec2<T> {
    fn new(x: T, y: T) -> Self {
        Vec2 { x, y }
    }
    fn yx(&self) -> Vec2<T> {
        Vec2 { x: self.y, y: self.x }
    }
}

impl Vec2<f64> {
    fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

impl Vec2<i32> {
    fn manhattan(&self) -> i32 {
        self.x.abs() + self.y.abs()
    }
}

impl Vec2<bool> {
    fn both(&self) -> bool {
        self.x && self.y
    }
}

struct Counter<T> {
    val: T,
    steps: u32,
}

impl Counter<u8> {
    fn bump(&mut self) {
        self.val = self.val.wrapping_add(100);
        self.steps += 1;
    }
}

impl Counter<i64> {
    fn bump(&mut self) {
        self.val -= 1000;
        self.steps += 1;
    }
}

fn main() {
    let a = Vec2::new(3.0, 4.0);
    println!("{}", a.length());
    let b = a.yx();
    println!("{} {}", b.x, b.y);
    let c = Vec2::new(-3, 7);
    println!("{}", c.manhattan());
    println!("{}", c.yx().x);
    let d = Vec2::new(true, false);
    println!("{} {}", d.both(), d.yx().both());
    let e = Vec2::new(true, true);
    println!("{}", e.both());

    let mut cu = Counter { val: 0u8, steps: 0 };
    for _ in 0..5 {
        cu.bump();
        println!("u8 {} {}", cu.val, cu.steps);
    }
    let mut ci = Counter { val: 0i64, steps: 0 };
    ci.bump();
    ci.bump();
    println!("i64 {} {}", ci.val, ci.steps);
}
