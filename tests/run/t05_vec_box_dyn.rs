// Vec<Box<dyn Trait>>: plugin-style pipeline, mutation through iter_mut, removal.
trait Stage {
    fn name(&self) -> &str;
    fn run(&mut self, x: i64) -> i64;
    fn calls(&self) -> u32;
}

struct AddK {
    k: i64,
    n: u32,
}
struct MulK {
    k: i64,
    n: u32,
}
struct Clamp {
    lo: i64,
    hi: i64,
    n: u32,
}

impl Stage for AddK {
    fn name(&self) -> &str {
        "add"
    }
    fn run(&mut self, x: i64) -> i64 {
        self.n += 1;
        x + self.k
    }
    fn calls(&self) -> u32 {
        self.n
    }
}
impl Stage for MulK {
    fn name(&self) -> &str {
        "mul"
    }
    fn run(&mut self, x: i64) -> i64 {
        self.n += 1;
        x * self.k
    }
    fn calls(&self) -> u32 {
        self.n
    }
}
impl Stage for Clamp {
    fn name(&self) -> &str {
        "clamp"
    }
    fn run(&mut self, x: i64) -> i64 {
        self.n += 1;
        if x < self.lo {
            self.lo
        } else if x > self.hi {
            self.hi
        } else {
            x
        }
    }
    fn calls(&self) -> u32 {
        self.n
    }
}

fn run_all(stages: &mut Vec<Box<dyn Stage>>, x: i64) -> i64 {
    let mut v = x;
    for s in stages.iter_mut() {
        v = s.run(v);
    }
    v
}

fn main() {
    let mut stages: Vec<Box<dyn Stage>> = vec![
        Box::new(AddK { k: 3, n: 0 }),
        Box::new(MulK { k: -4, n: 0 }),
        Box::new(Clamp { lo: -50, hi: 50, n: 0 }),
    ];
    for x in [0, 5, 10, 20, -30] {
        println!("{} -> {}", x, run_all(&mut stages, x));
    }
    stages.push(Box::new(AddK { k: 1000, n: 0 }));
    println!("{}", run_all(&mut stages, 1));
    for s in &stages {
        println!("{} called {}", s.name(), s.calls());
    }
    let removed = stages.remove(1);
    println!("removed {} ({})", removed.name(), removed.calls());
    println!("{}", run_all(&mut stages, 100));
    stages.reverse();
    println!("{}", run_all(&mut stages, 100));
    let names: Vec<&str> = stages.iter().map(|s| s.name()).collect();
    println!("{:?}", names);
    let total: u32 = stages.iter().map(|s| s.calls()).sum();
    println!("{}", total);
}
