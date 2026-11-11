fn find_first_neg(xs: &[i32]) -> Option<usize> {
    let mut i = 0;
    while i < xs.len() {
        if xs[i] < 0 {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn div(a: i32, b: i32) -> Option<i32> {
    if b == 0 { None } else { Some(a / b) }
}

fn show(o: Option<i32>) {
    match o {
        Some(v) if v > 100 => println!("big {}", v),
        Some(v) => println!("some {}", v),
        None => println!("none"),
    }
}

fn main() {
    let a = [3, 4, -1, 5, -9];
    let b = [1, 2, 3];
    match find_first_neg(&a) {
        Some(i) => println!("neg at {} = {}", i, a[i]),
        None => println!("no neg"),
    }
    match find_first_neg(&b) {
        Some(i) => println!("neg at {}", i),
        None => println!("no neg"),
    }
    show(div(10, 3));
    show(div(1000, 2));
    show(div(1, 0));
    let x: Option<u8> = Some(255);
    let y: Option<u8> = None;
    println!("{} {} {} {}", x.is_some(), x.is_none(), y.is_some(), y.is_none());
    if let Some(v) = x {
        println!("x {}", v);
    }
    let nested: Option<Option<i32>> = Some(None);
    match nested {
        Some(Some(v)) => println!("inner {}", v),
        Some(None) => println!("some none"),
        None => println!("none"),
    }
    let mut counter = Some(0);
    while let Some(c) = counter {
        println!("c {}", c);
        counter = if c < 4 { Some(c + 1) } else { None };
    }
    let sz = std::mem::size_of::<Option<u32>>() >= 4;
    println!("{}", sz);
    let arr = [Some(1), None, Some(3)];
    let mut sum = 0;
    for i in 0..arr.len() {
        if let Some(v) = arr[i] {
            sum += v;
        }
    }
    println!("sum {}", sum);
}
