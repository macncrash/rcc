// Returning closures: impl Fn, Box<dyn Fn>, closures returning closures, composition.
fn adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

fn op(name: &str) -> Box<dyn Fn(i32, i32) -> i32> {
    match name {
        "add" => Box::new(|a, b| a + b),
        "mul" => Box::new(|a, b| a * b),
        "pow" => Box::new(|a, b| {
            let mut r = 1;
            for _ in 0..b {
                r *= a;
            }
            r
        }),
        _ => Box::new(|_, _| 0),
    }
}

fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

fn curry(a: i32) -> impl Fn(i32) -> Box<dyn Fn(i32) -> i32> {
    move |b| Box::new(move |c| a * 100 + b * 10 + c)
}

fn counter_from(start: u32) -> Box<dyn FnMut() -> u32> {
    let mut n = start;
    Box::new(move || {
        let r = n;
        n += 1;
        r
    })
}

fn main() {
    let a5 = adder(5);
    let a10 = adder(10);
    println!("{} {}", a5(1), a10(1));
    for name in ["add", "mul", "pow", "nope"] {
        println!("{} {}", name, op(name)(3, 4));
    }
    let f = compose(|x: i32| x + 1, |y: i32| y * 10);
    println!("{}", f(4));
    let g = compose(|s: &str| s.len(), |n: usize| n % 2 == 0);
    println!("{} {}", g("ab"), g("abc"));
    let h = compose(compose(a5, a10), |z: i32| format!("[{}]", z));
    println!("{}", h(0));
    println!("{}", curry(1)(2)(3));
    let c4 = curry(4);
    let c45 = c4(5);
    println!("{} {}", c45(6), c45(7));
    let mut ctr = counter_from(40);
    println!("{} {} {}", ctr(), ctr(), ctr());
    let mut fs: Vec<Box<dyn Fn(i32) -> i32>> = Vec::new();
    for k in 1..=3 {
        fs.push(Box::new(adder(k * 100)));
    }
    fs.push(op("mul").as_ref().clone_box_hack());
    for f in &fs {
        print!("{} ", f(1));
    }
    println!();
}

// helper trait to show a method on dyn Fn via a blanket impl
trait CloneBoxHack {
    fn clone_box_hack(&self) -> Box<dyn Fn(i32) -> i32>;
}
impl CloneBoxHack for dyn Fn(i32, i32) -> i32 {
    fn clone_box_hack(&self) -> Box<dyn Fn(i32) -> i32> {
        let v = self(6, 7);
        Box::new(move |x| x + v)
    }
}
