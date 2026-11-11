// Default methods, overriding defaults, defaults calling required methods.
trait Animal {
    fn name(&self) -> &'static str;
    fn legs(&self) -> u32 {
        4
    }
    fn noise(&self) -> &'static str {
        "..."
    }
    fn describe(&self) {
        println!("{} has {} legs and says {}", self.name(), self.legs(), self.noise());
    }
    fn total_legs(&self, n: u32) -> u32 {
        self.legs() * n
    }
}

struct Dog;
struct Bird;
struct Fish {
    kind: &'static str,
}

impl Animal for Dog {
    fn name(&self) -> &'static str {
        "dog"
    }
    fn noise(&self) -> &'static str {
        "woof"
    }
}

impl Animal for Bird {
    fn name(&self) -> &'static str {
        "bird"
    }
    fn legs(&self) -> u32 {
        2
    }
    fn noise(&self) -> &'static str {
        "tweet"
    }
}

impl Animal for Fish {
    fn name(&self) -> &'static str {
        self.kind
    }
    fn legs(&self) -> u32 {
        0
    }
    fn describe(&self) {
        println!("{} swims silently", self.name());
    }
}

trait Accum {
    fn step(&self) -> i64;
    fn start(&self) -> i64 {
        0
    }
    fn run(&self, n: i64) -> i64 {
        let mut v = self.start();
        let mut i = 0;
        while i < n {
            v += self.step();
            i += 1;
        }
        v
    }
}

struct Ones;
struct Neg {
    from: i64,
}
impl Accum for Ones {
    fn step(&self) -> i64 {
        1
    }
}
impl Accum for Neg {
    fn step(&self) -> i64 {
        -3
    }
    fn start(&self) -> i64 {
        self.from
    }
}

fn show<A: Animal>(a: &A) {
    a.describe();
    println!("ten of them: {}", a.total_legs(10));
}

fn main() {
    show(&Dog);
    show(&Bird);
    show(&Fish { kind: "trout" });
    println!("{}", Ones.run(5));
    println!("{}", Neg { from: 100 }.run(5));
    println!("{}", Neg { from: 1 }.start());
}
