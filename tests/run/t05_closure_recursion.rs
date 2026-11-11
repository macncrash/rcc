// Recursion with closures and fn pointers: passing &dyn Fn to itself,
// memoization through &mut captured state, Y-combinator style.
fn fix(f: &dyn Fn(&dyn Fn(u64) -> u64, u64) -> u64, n: u64) -> u64 {
    f(&|k| fix(f, k), n)
}

fn memo_fib(n: usize, memo: &mut Vec<Option<u64>>) -> u64 {
    if let Some(v) = memo[n] {
        return v;
    }
    let v = if n < 2 { n as u64 } else { memo_fib(n - 1, memo) + memo_fib(n - 2, memo) };
    memo[n] = Some(v);
    v
}

fn walk_tree(depth: u32, f: &mut dyn FnMut(u32, u32)) {
    fn go(d: u32, idx: u32, max: u32, f: &mut dyn FnMut(u32, u32)) {
        f(d, idx);
        if d < max {
            go(d + 1, idx * 2, max, f);
            go(d + 1, idx * 2 + 1, max, f);
        }
    }
    go(0, 1, depth, f);
}

struct Rec<'a> {
    f: &'a dyn Fn(&Rec, i32) -> i32,
}
impl<'a> Rec<'a> {
    fn call(&self, x: i32) -> i32 {
        (self.f)(self, x)
    }
}

fn main() {
    let fact = |rec: &dyn Fn(u64) -> u64, n: u64| if n == 0 { 1 } else { n * rec(n - 1) };
    println!("{}", fix(&fact, 10));
    let fib = |rec: &dyn Fn(u64) -> u64, n: u64| if n < 2 { n } else { rec(n - 1) + rec(n - 2) };
    println!("{}", fix(&fib, 20));
    let mut memo = vec![None; 91];
    println!("{}", memo_fib(90, &mut memo));
    println!("{}", memo.iter().filter(|m| m.is_some()).count());
    let mut seen = Vec::new();
    walk_tree(3, &mut |d, i| seen.push(format!("{}:{}", d, i)));
    println!("{}", seen.join(" "));
    let mut sum = 0;
    walk_tree(10, &mut |_, i| sum += i);
    println!("{}", sum);
    let collatz = Rec {
        f: &|r, x| {
            if x == 1 {
                0
            } else if x % 2 == 0 {
                1 + r.call(x / 2)
            } else {
                1 + r.call(3 * x + 1)
            }
        },
    };
    for x in [1, 6, 7, 27] {
        println!("collatz {} {}", x, collatz.call(x));
    }
}
