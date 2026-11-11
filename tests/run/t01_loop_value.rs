fn find_first_divisor(n: u32) -> u32 {
    let mut d = 2;
    loop {
        if d * d > n {
            break n;
        }
        if n % d == 0 {
            break d;
        }
        d += 1;
    }
}

fn main() {
    for n in [15, 17, 91, 97, 1001, 7919] {
        println!("smallest divisor of {} = {}", n, find_first_divisor(n));
    }
    let mut counter = 0;
    let pair = loop {
        counter += 3;
        if counter > 20 {
            break (counter, counter * 2);
        }
    };
    println!("{} {}", pair.0, pair.1);
    let mut x = 1.0f64;
    let iters = loop {
        x *= 1.1;
        if x > 100.0 {
            break counter;
        }
        counter += 1;
    };
    println!("iters {} x {}", iters, x);
    let flag = loop {
        break true;
    };
    println!("{}", flag);
    let arr = loop {
        break [1, 2, 3];
    };
    println!("{}", arr[0] + arr[1] + arr[2]);
    let mut k = 0;
    let s = loop {
        k += 1;
        let inner = loop {
            if k % 4 == 0 {
                break k * 10;
            }
            break 0;
        };
        if inner != 0 {
            break inner + 1;
        }
    };
    println!("s {}", s);
}
