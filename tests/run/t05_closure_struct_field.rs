// Closures stored in struct fields: generic F field, Box<dyn Fn>, Option<Box<dyn FnMut>>.
struct Cache<F: Fn(u64) -> u64> {
    f: F,
    hits: u32,
    last: Option<(u64, u64)>,
}

impl<F: Fn(u64) -> u64> Cache<F> {
    fn new(f: F) -> Self {
        Cache { f, hits: 0, last: None }
    }
    fn get(&mut self, x: u64) -> u64 {
        if let Some((k, v)) = self.last {
            if k == x {
                self.hits += 1;
                return v;
            }
        }
        let v = (self.f)(x);
        self.last = Some((x, v));
        v
    }
}

struct Button {
    label: &'static str,
    on_click: Box<dyn Fn(&str) -> String>,
}

struct Machine {
    hook: Option<Box<dyn FnMut(i32) -> i32>>,
    state: i32,
}

impl Machine {
    fn step(&mut self) {
        self.state = match self.hook.as_mut() {
            Some(h) => h(self.state),
            None => self.state + 1,
        };
    }
}

struct Ops {
    ops: Vec<(&'static str, fn(i32, i32) -> i32)>,
}

fn main() {
    let mut c = Cache::new(|x| {
        println!("  computing {}", x);
        x * x + 1
    });
    for x in [3, 3, 4, 4, 4, 3] {
        println!("{} -> {}", x, c.get(x));
    }
    println!("hits {}", c.hits);

    let prefix = String::from(">>");
    let buttons = vec![
        Button { label: "ok", on_click: Box::new(|l| format!("clicked {}", l)) },
        Button { label: "cancel", on_click: Box::new(move |l| format!("{} {}", prefix, l.to_uppercase())) },
    ];
    for b in &buttons {
        println!("{}", (b.on_click)(b.label));
    }

    let mut m = Machine { hook: None, state: 0 };
    m.step();
    m.step();
    println!("{}", m.state);
    let mut calls = 0;
    m.hook = Some(Box::new(move |s| {
        calls += 1;
        s * 2 + calls
    }));
    m.step();
    m.step();
    m.step();
    println!("{}", m.state);
    m.hook = None;
    m.step();
    println!("{}", m.state);

    let o = Ops { ops: vec![("add", |a, b| a + b), ("sub", |a, b| a - b), ("mul", |a, b| a * b)] };
    for (n, f) in &o.ops {
        println!("{} {}", n, f(7, 3));
    }
}
