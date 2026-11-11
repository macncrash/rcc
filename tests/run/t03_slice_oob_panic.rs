fn sum_to(xs: &[i64], n: usize) -> i64 {
    let mut s = 0;
    for i in 0..n {
        s += xs[i];
        println!("i {} s {}", i, s);
    }
    s
}

fn main() {
    let a = [5i64, 6, 7, 8];
    println!("{}", sum_to(&a, 4));
    let sl = &a[1..3];
    println!("len {}", sl.len());
    println!("{}", sum_to(sl, 3));
    println!("never");
}
