// Closures: no captures, typed/untyped params, block bodies, immediate calls.
fn main() {
    let add = |a, b| a + b;
    println!("{}", add(2, 3));
    let sq = |x: i64| -> i64 { x * x };
    println!("{}", sq(-9));
    let unit = || println!("called unit closure");
    unit();
    unit();
    let pick = |c: bool, a: &'static str, b: &'static str| if c { a } else { b };
    println!("{} {}", pick(true, "yes", "no"), pick(false, "yes", "no"));
    let fact = |n: u64| {
        let mut r = 1;
        let mut i = 2;
        while i <= n {
            r *= i;
            i += 1;
        }
        r
    };
    println!("{} {}", fact(10), fact(20));
    println!("{}", (|x: i32| x * 100)(7));
    let tuple_in = |(a, b): (i32, i32)| a * b;
    println!("{}", tuple_in((6, 7)));
    let make_pair = |x| (x, x + 1);
    let (p, q) = make_pair(41u8);
    println!("{} {}", p, q);
    let nested = |x: i32| {
        let inner = |y: i32| y * 2 + 1;
        inner(inner(x))
    };
    println!("{}", nested(3));
    let fs = [|x: i32| x + 1, |x: i32| x + 1];
    println!("{}", fs[0](1) + fs[1](2));
    let ret_closure_result = {
        let f = |s: &str| s.len();
        f("hello") + f("")
    };
    println!("{}", ret_closure_result);
    let early = |x: i32| -> &'static str {
        if x < 0 {
            return "neg";
        }
        if x == 0 {
            return "zero";
        }
        "pos"
    };
    println!("{} {} {}", early(-1), early(0), early(5));
}
