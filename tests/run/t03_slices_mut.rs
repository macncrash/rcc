fn bump(xs: &mut [i32], d: i32) {
    for i in 0..xs.len() {
        xs[i] += d;
    }
}

fn bubble(xs: &mut [i32]) -> u32 {
    let mut swaps = 0;
    let n = xs.len();
    for i in 0..n {
        for j in 0..n - 1 - i {
            if xs[j] > xs[j + 1] {
                xs.swap(j, j + 1);
                swaps += 1;
            }
        }
    }
    swaps
}

fn reverse(xs: &mut [i32]) {
    if xs.len() < 2 {
        return;
    }
    let mut i = 0;
    let mut j = xs.len() - 1;
    while i < j {
        let t = xs[i];
        xs[i] = xs[j];
        xs[j] = t;
        i += 1;
        j -= 1;
    }
}

fn show(xs: &[i32]) {
    for i in 0..xs.len() {
        print!("{} ", xs[i]);
    }
    println!();
}

fn main() {
    let mut a = [9, 3, 7, 1, 8, 2, 6, 4, 5, 0];
    show(&a);
    bump(&mut a[..5], 100);
    show(&a);
    let swaps = bubble(&mut a);
    println!("swaps {}", swaps);
    show(&a);
    reverse(&mut a[2..8]);
    show(&a);
    {
        let (l, r) = a.split_at_mut(5);
        l[0] = r[0] * 2;
        r[4] = l[1] + l[2];
    }
    show(&a);
    a.reverse();
    show(&a);
    a.sort();
    show(&a);
    a[3..6].fill(-1);
    show(&a);
    if let Some(f) = a.first_mut() {
        *f = 42;
    }
    if let Some(l) = a.last_mut() {
        *l = -42;
    }
    show(&a);
    let mut b = [0; 4];
    b.copy_from_slice(&a[1..5]);
    show(&b);
    let s: &mut [i32] = &mut b;
    s[0] = 7;
    show(&b);
}
