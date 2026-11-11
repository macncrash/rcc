fn main() {
    let a = [10, 20, 30, 40];
    for x in a {
        print!("{} ", x);
    }
    println!();
    for x in &a {
        print!("{} ", *x + 1);
    }
    println!();
    let mut b = [1, 2, 3, 4];
    for x in &mut b {
        *x *= *x;
    }
    for x in b {
        print!("{} ", x);
    }
    println!();
    for i in 0..a.len() {
        print!("{}:{} ", i, a[i]);
    }
    println!();
    let nested = [[1, 2], [3, 4], [5, 6]];
    for row in &nested {
        for v in row {
            print!("{} ", v);
        }
        print!("| ");
    }
    println!();
    let mut sum = 0;
    for x in a.iter() {
        sum += x;
    }
    println!("sum {}", sum);
    for i in (0..a.len()).rev() {
        print!("{} ", a[i]);
    }
    println!();
    let mut last = 0;
    for &x in &a {
        if x > 25 {
            last = x;
            break;
        }
    }
    println!("first > 25: {}", last);
    let words = ["alpha", "beta", "gamma"];
    for w in words {
        println!("{} has {} bytes", w, w.len());
    }
}
