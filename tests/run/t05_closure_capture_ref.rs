// Closures capturing by shared reference; captured values seen after mutation
// ends; multiple closures sharing a capture.
struct Config {
    scale: i32,
    offset: i32,
}

fn main() {
    let base = 10;
    let add_base = |x: i32| x + base;
    println!("{} {}", add_base(1), add_base(-10));
    let name = String::from("world");
    let greet = || format!("hello {}", name);
    println!("{}", greet());
    println!("{} still usable, len {}", name, name.len());
    let cfg = Config { scale: 3, offset: -1 };
    let apply = |x: i32| x * cfg.scale + cfg.offset;
    let apply2 = |x: i32| apply(apply(x));
    println!("{} {}", apply(5), apply2(5));
    let arr = [5, 1, 4, 2, 3];
    let count_below = |t: i32| {
        let mut c = 0;
        for &v in arr.iter() {
            if v < t {
                c += 1;
            }
        }
        c
    };
    for t in 0..7 {
        print!("{} ", count_below(t));
    }
    println!();
    let mut total = 0;
    {
        let peek = || total + 1;
        println!("peek {}", peek());
    }
    total += 5;
    let peek2 = || total * 2;
    println!("peek2 {}", peek2());
    let v = vec![String::from("a"), String::from("bb"), String::from("ccc")];
    let longest = || {
        let mut best = &v[0];
        for s in &v {
            if s.len() > best.len() {
                best = s;
            }
        }
        best.clone()
    };
    println!("{} {}", longest(), v.len());
    // closure capturing a reference variable
    let r = &arr;
    let first = || r[0];
    println!("{}", first());
}
