// FnOnce: closures that move captured values out, Option<FnOnce> take-and-call,
// deferred work queues.
struct Task {
    name: String,
    job: Option<Box<dyn FnOnce() -> String>>,
}

impl Task {
    fn run(&mut self) -> String {
        match self.job.take() {
            Some(j) => format!("{}: {}", self.name, j()),
            None => format!("{}: already ran", self.name),
        }
    }
}

fn consume_with<F: FnOnce(Vec<i32>) -> usize>(v: Vec<i32>, f: F) -> usize {
    f(v)
}

fn call_once<T, F: FnOnce() -> T>(f: F) -> T {
    f()
}

fn main() {
    let words = vec![String::from("alpha"), String::from("beta")];
    let joiner = move || words.join("+");
    println!("{}", joiner());

    let data = String::from("payload");
    let mut t = Task { name: String::from("t1"), job: Some(Box::new(move || data + "!")) };
    println!("{}", t.run());
    println!("{}", t.run());

    let v = vec![1, 2, 3];
    let n = consume_with(v, |mut v| {
        v.push(4);
        v.len()
    });
    println!("{}", n);

    let mut queue: Vec<Box<dyn FnOnce(&mut Vec<String>)>> = Vec::new();
    for i in 0..3 {
        let owned = format!("job{}", i);
        queue.push(Box::new(move |out| out.push(owned)));
    }
    let mut out = Vec::new();
    for job in queue {
        job(&mut out);
    }
    println!("{:?}", out);

    let s = String::from("moved");
    let r = call_once(move || {
        let mut s = s;
        s.push_str(" twice");
        s
    });
    println!("{}", r);
    let pair = (String::from("l"), String::from("r"));
    let f = move || {
        let (a, b) = pair;
        b + &a
    };
    println!("{}", call_once(f));
    let opt: Option<Box<dyn FnOnce(i32) -> i32>> = Some(Box::new(|x| x * 11));
    if let Some(g) = opt {
        println!("{}", g(3));
    }
}
