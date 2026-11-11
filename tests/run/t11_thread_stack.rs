// thread::Builder with a large stack, deep recursion inside it, and join results
use std::thread;

fn depth(n: u64, acc: &mut [u64; 4]) -> u64 {
    // keep a frame large enough that recursion really uses stack
    let local = [n, n ^ 1, n.wrapping_mul(3), n + 7];
    acc[(n % 4) as usize] = acc[(n % 4) as usize].wrapping_add(local[(n % 3) as usize]);
    if n == 0 {
        0
    } else {
        1 + depth(n - 1, acc)
    }
}

fn ackermann(m: u64, n: u64) -> u64 {
    if m == 0 {
        n + 1
    } else if n == 0 {
        ackermann(m - 1, 1)
    } else {
        ackermann(m - 1, ackermann(m, n - 1))
    }
}

fn main() {
    let h = thread::Builder::new()
        .name("deep".to_string())
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            let mut acc = [0u64; 4];
            let d = depth(100_000, &mut acc);
            let name = thread::current().name().map(|s| s.to_string());
            (d, acc, name)
        })
        .unwrap();
    let (d, acc, name) = h.join().unwrap();
    println!("depth {} acc {:?} name {:?}", d, acc, name);

    let h2 = thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| ackermann(2, 3000))
        .unwrap();
    println!("ackermann(2, 3000) = {}", h2.join().unwrap());

    let mut handles = Vec::new();
    for i in 0..4u64 {
        handles.push(thread::spawn(move || {
            let mut s = 0u64;
            for k in 0..=i * 1000 {
                s += k;
            }
            (i, s)
        }));
    }
    for h in handles {
        let (i, s) = h.join().unwrap();
        println!("thread {} sum {}", i, s);
    }
    let data = vec![1, 2, 3];
    let r = thread::scope(|sc| {
        let a = sc.spawn(|| data.iter().sum::<i32>());
        let b = sc.spawn(|| data.len());
        (a.join().unwrap(), b.join().unwrap())
    });
    println!("scoped {:?}", r);
}
