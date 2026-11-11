struct Counter {
    count: u32,
    step: u32,
}

impl Counter {
    const START: u32 = 10;

    fn new(step: u32) -> Counter {
        Counter { count: Self::START, step }
    }

    fn tick(&mut self) -> u32 {
        self.count += self.step;
        self.count
    }

    fn get(&self) -> u32 {
        self.count
    }

    fn reset(&mut self) {
        self.count = Self::START;
    }

    fn into_total(self) -> u32 {
        self.count + self.step
    }

    fn bigger(&self, other: &Counter) -> bool {
        self.count > other.count
    }
}

impl Counter {
    fn twice(&mut self) -> &mut Self {
        self.tick();
        self.tick();
        self
    }
}

fn main() {
    let mut c = Counter::new(3);
    println!("start {}", c.get());
    for _ in 0..4 {
        println!("tick {}", c.tick());
    }
    c.reset();
    println!("reset {}", c.get());
    c.twice().twice();
    println!("after twice twice {}", c.get());
    let d = Counter::new(100);
    println!("bigger {} {}", c.bigger(&d), d.bigger(&c));
    let r = &mut c;
    r.tick();
    println!("via ref {}", r.get());
    println!("ufcs {}", Counter::get(&c));
    println!("total {}", c.into_total());
    println!("const {}", Counter::START);
}
