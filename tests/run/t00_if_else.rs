fn classify(n: i32) -> i32 {
    if n < 0 {
        -1
    } else if n == 0 {
        0
    } else if n < 10 {
        1
    } else if n < 100 {
        2
    } else {
        3
    }
}

fn main() {
    let mut n = -5;
    while n < 150 {
        println!("{} -> {}", n, classify(n));
        n += 13;
    }
    let a = 3;
    let b = if a > 2 { a * 10 } else { a - 10 };
    println!("b = {}", b);
    let c = if a == 3 && b == 30 { 1 } else { 0 };
    println!("c = {}", c);
    if a != 3 || b != 30 {
        println!("wrong");
    } else {
        println!("right");
    }
    if !(a > 5) {
        println!("not greater than 5");
    }
    let max = if a > b { a } else { b };
    println!("max = {}", max);
    let nested = if a > 0 { if b > 100 { 1 } else { 2 } } else { 3 };
    println!("nested = {}", nested);
    if a == 3 {
        println!("statement if");
    }
}
