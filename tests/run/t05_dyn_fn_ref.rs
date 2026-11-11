// &dyn Fn, &mut dyn FnMut, Box<dyn FnOnce> as parameters.
fn run(f: &dyn Fn(i32) -> i32, xs: &[i32]) -> Vec<i32> {
    let mut out = Vec::new();
    for &x in xs {
        out.push(f(x));
    }
    out
}

fn each(xs: &[&str], f: &mut dyn FnMut(usize, &str)) {
    for (i, x) in xs.iter().enumerate() {
        f(i, x);
    }
}

fn finish(f: Box<dyn FnOnce(String) -> String>) -> String {
    f(String::from("seed"))
}

fn walk(n: u32, visit: &mut dyn FnMut(u32)) {
    if n == 0 {
        return;
    }
    visit(n);
    walk(n / 2, visit);
    walk(n / 3, visit);
}

fn choose<'a>(flag: bool, a: &'a dyn Fn() -> &'static str, b: &'a dyn Fn() -> &'static str) -> &'a dyn Fn() -> &'static str {
    if flag { a } else { b }
}

fn main() {
    let k = 3;
    println!("{:?}", run(&|x| x * k, &[1, 2, 3]));
    println!("{:?}", run(&|x| -x, &[5, -5]));
    fn plus_one(x: i32) -> i32 {
        x + 1
    }
    println!("{:?}", run(&plus_one, &[0, 9]));

    let mut joined = String::new();
    each(&["a", "b", "c"], &mut |i, s| {
        joined.push_str(&format!("{}={};", i, s));
    });
    println!("{}", joined);

    let suffix = String::from("-done");
    println!("{}", finish(Box::new(move |s| s + &suffix)));
    println!("{}", finish(Box::new(|s| s.to_uppercase())));

    let mut visited = Vec::new();
    walk(12, &mut |n| visited.push(n));
    println!("{:?}", visited);
    let mut total = 0;
    let mut add = |n: u32| total += n;
    walk(30, &mut add);
    println!("{}", total);

    let yes = || "yes";
    let no = || "no";
    println!("{} {}", choose(true, &yes, &no)(), choose(false, &yes, &no)());
}
