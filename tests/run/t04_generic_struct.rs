// Generic structs with impl blocks, including multiple type params.
struct Pair<T> {
    a: T,
    b: T,
}

impl<T: PartialOrd + Copy> Pair<T> {
    fn new(a: T, b: T) -> Self {
        Pair { a, b }
    }
    fn max(&self) -> T {
        if self.a > self.b { self.a } else { self.b }
    }
    fn swap(&mut self) {
        let t = self.a;
        self.a = self.b;
        self.b = t;
    }
}

struct Point<T, U> {
    x: T,
    y: U,
}

impl<T: Copy, U: Copy> Point<T, U> {
    fn mixup<V: Copy, W: Copy>(&self, other: &Point<V, W>) -> Point<T, W> {
        Point { x: self.x, y: other.y }
    }
    fn x(&self) -> T {
        self.x
    }
    fn y(&self) -> U {
        self.y
    }
}

struct Wrapper<T>(T);

impl<T> Wrapper<T> {
    fn into_inner(self) -> T {
        self.0
    }
    fn get(&self) -> &T {
        &self.0
    }
}

struct Stack<T> {
    items: [T; 8],
    len: usize,
}

impl<T: Copy + Default> Stack<T> {
    fn new() -> Self {
        Stack { items: [T::default(); 8], len: 0 }
    }
    fn push(&mut self, v: T) {
        self.items[self.len] = v;
        self.len += 1;
    }
    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(self.items[self.len])
        }
    }
}

fn main() {
    let mut p = Pair::new(3, 8);
    println!("{} {} max={}", p.a, p.b, p.max());
    p.swap();
    println!("{} {} max={}", p.a, p.b, p.max());
    let q = Pair::new(2.5, -1.0);
    println!("max={}", q.max());
    let pc = Pair::new('m', 'k');
    println!("max={}", pc.max());

    let p1 = Point { x: 5i32, y: 10.5f64 };
    let p2 = Point { x: 'c', y: true };
    let p3 = p1.mixup(&p2);
    println!("{} {}", p3.x(), p3.y());
    println!("{} {}", p1.x(), p1.y());
    println!("{} {}", p2.x, p2.y);

    let w = Wrapper(42u64);
    println!("{}", w.get());
    println!("{}", w.into_inner() + 1);
    let w2 = Wrapper((1, 2));
    println!("{}", w2.get().0 + w2.get().1);

    let mut s: Stack<i32> = Stack::new();
    for i in 0..5 {
        s.push(i * i);
    }
    while let Some(v) = s.pop() {
        println!("pop {}", v);
    }
    let mut sb = Stack::<bool>::new();
    sb.push(true);
    sb.push(false);
    println!("{}", sb.len);
    println!("{}", sb.pop().unwrap());
    println!("{}", sb.pop().unwrap());
    println!("{}", sb.pop().is_none());
}
