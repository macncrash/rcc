// FnMut closures: mutating captured variables, counters, accumulating into Vec.
fn call_n<F: FnMut()>(n: u32, mut f: F) {
    for _ in 0..n {
        f();
    }
}

fn main() {
    let mut count = 0;
    let mut inc = || count += 1;
    inc();
    inc();
    inc();
    println!("count {}", count);

    let mut log = Vec::new();
    let mut record = |s: &str| log.push(s.to_string());
    record("a");
    record("b");
    record("c");
    println!("{:?}", log);

    let mut sum = 0i64;
    call_n(10, || sum += 7);
    println!("sum {}", sum);

    let mut fib = (0u64, 1u64);
    let mut next = || {
        let r = fib.0;
        fib = (fib.1, fib.0 + fib.1);
        r
    };
    for _ in 0..12 {
        print!("{} ", next());
    }
    println!();
    println!("state {:?}", fib);

    let mut arr = [1, 2, 3, 4];
    let mut double_at = |i: usize| arr[i] *= 2;
    double_at(0);
    double_at(3);
    double_at(3);
    println!("{:?}", arr);

    struct S {
        a: i32,
        b: i32,
    }
    let mut s = S { a: 1, b: 100 };
    {
        let mut bump_a = || s.a += 10;
        bump_a();
        bump_a();
    }
    let read_b = || s.b;
    println!("{} {}", s.a, read_b());

    let mut text = String::new();
    let mut append = |c: char, n: usize| {
        for _ in 0..n {
            text.push(c);
        }
        text.len()
    };
    println!("{}", append('x', 3));
    println!("{}", append('y', 2));
    println!("{}", text);
}
