fn main() {
    let mut i = 0;
    loop {
        i += 1;
        if i % 2 == 0 {
            continue;
        }
        println!("odd {}", i);
        if i > 10 {
            break;
        }
    }
    let mut n = 27;
    let mut steps = 0;
    let result = loop {
        if n == 1 {
            break steps;
        }
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        steps += 1;
    };
    println!("collatz 27 steps = {}", result);
    let mut count = 0;
    loop {
        count += 1;
        let mut inner = 0;
        loop {
            inner += 1;
            if inner == count {
                break;
            }
        }
        println!("count {} inner {}", count, inner);
        if count == 4 {
            break;
        }
    }
    let first_square_over = {
        let mut k = 0;
        loop {
            if k * k > 500 {
                break k;
            }
            k += 1;
        }
    };
    println!("first square over 500: {}", first_square_over);
}
