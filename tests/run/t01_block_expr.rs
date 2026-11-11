fn compute(n: i32) -> i32 {
    let a = {
        let t = n * 2;
        t + 1
    };
    let b = if a > 10 {
        let u = a - 10;
        u * u
    } else {
        0
    };
    {
        println!("a={} b={}", a, b);
    }
    a + b
}

fn main() {
    println!("{}", compute(3));
    println!("{}", compute(8));
    let v = {
        let mut s = 0;
        for i in 0..10 {
            s += i;
        }
        s
    };
    println!("v={}", v);
    let unit = {
        let _ = 5;
    };
    let () = unit;
    let nested = {
        let a = {
            let b = {
                let c = 3;
                c * 3
            };
            b + 1
        };
        a * 2
    };
    println!("nested={}", nested);
    let x = 7;
    let parity = { if x % 2 == 0 { "even" } else { "odd" } };
    println!("{} is {}", x, parity);
    let t = { (1, 2) };
    println!("{} {}", t.0, t.1);
    let r = 1 + { 2 * 3 } + { let q = 4; q };
    println!("r={}", r);
    let early = 'b: {
        if x > 5 {
            break 'b x * 100;
        }
        x
    };
    println!("early={}", early);
    let arr = [{ 1 + 1 }, { 2 * 2 }, { let z = 3; z * z * z }];
    println!("{} {} {}", arr[0], arr[1], arr[2]);
    let mut counter = 0;
    let w = loop {
        counter += 1;
        let done = { counter * counter > 50 };
        if done {
            break { let c = counter; c - 1 };
        }
    };
    println!("w={}", w);
    unsafe_free();
}

fn unsafe_free() {
    let value = { 10 } * { 2 };
    println!("value={}", value);
}
