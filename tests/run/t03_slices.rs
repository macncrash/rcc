fn sum(xs: &[i32]) -> i32 {
    let mut s = 0;
    for i in 0..xs.len() {
        s += xs[i];
    }
    s
}

fn max(xs: &[i32]) -> Option<i32> {
    if xs.is_empty() {
        return None;
    }
    let mut m = xs[0];
    for i in 1..xs.len() {
        if xs[i] > m {
            m = xs[i];
        }
    }
    Some(m)
}

fn show(xs: &[i32]) {
    print!("[");
    for i in 0..xs.len() {
        if i > 0 {
            print!(", ");
        }
        print!("{}", xs[i]);
    }
    println!("] len {}", xs.len());
}

fn main() {
    let a = [4, -2, 9, 0, 7, 3, 8, -5];
    show(&a);
    show(&a[2..5]);
    show(&a[..3]);
    show(&a[5..]);
    show(&a[..]);
    show(&a[3..3]);
    show(&a[1..=2]);
    println!("sum {} {}", sum(&a), sum(&a[4..]));
    println!("max {}", max(&a).unwrap());
    println!("empty max {}", max(&a[0..0]).is_none());
    let s: &[i32] = &a;
    println!("first {} last {}", s.first().unwrap(), s.last().unwrap());
    println!("get {} {}", s.get(2).unwrap(), s.get(100).is_none());
    let (l, r) = s.split_at(3);
    show(l);
    show(r);
    println!("contains {} {}", s.contains(&9), s.contains(&100));
    let nested = &s[1..7][2..4];
    show(nested);
    match s.split_first() {
        Some((h, t)) => println!("head {} tail len {}", h, t.len()),
        None => println!("empty"),
    }
    let e: &[i32] = &[];
    println!("{}", e.len());
}
