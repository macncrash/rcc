fn binary_search(xs: &[i32], target: i32) -> Option<usize> {
    let mut lo = 0;
    let mut hi = xs.len();
    let mut steps = 0;
    while lo < hi {
        steps += 1;
        let mid = lo + (hi - lo) / 2;
        if xs[mid] == target {
            println!("  found {} in {} steps", target, steps);
            return Some(mid);
        } else if xs[mid] < target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    println!("  missing {} after {} steps", target, steps);
    None
}

fn insertion_sort(xs: &mut [i32]) {
    for i in 1..xs.len() {
        let mut j = i;
        while j > 0 && xs[j - 1] > xs[j] {
            xs.swap(j - 1, j);
            j -= 1;
        }
    }
}

fn partition(xs: &mut [i32]) -> usize {
    let p = xs.len() - 1;
    let mut store = 0;
    for i in 0..p {
        if xs[i] < xs[p] {
            xs.swap(i, store);
            store += 1;
        }
    }
    xs.swap(store, p);
    store
}

fn quicksort(xs: &mut [i32]) {
    if xs.len() <= 1 {
        return;
    }
    let p = partition(xs);
    let (l, r) = xs.split_at_mut(p);
    quicksort(l);
    quicksort(&mut r[1..]);
}

fn show(xs: &[i32]) {
    for i in 0..xs.len() {
        print!("{} ", xs[i]);
    }
    println!();
}

fn main() {
    let mut a = [38, 27, 43, 3, 9, 82, 10, -4, 27, 0, 15, 61];
    let mut b = a;
    insertion_sort(&mut a);
    show(&a);
    quicksort(&mut b);
    show(&b);
    let qs = [27, 82, -4, 5, 100, -10];
    for i in 0..qs.len() {
        match binary_search(&a, qs[i]) {
            Some(idx) => println!("{} at {}", qs[i], idx),
            None => println!("{} absent", qs[i]),
        }
    }
    let mut big = [0i32; 64];
    let mut seed: u32 = 12345;
    for i in 0..big.len() {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        big[i] = ((seed >> 16) % 1000) as i32;
    }
    quicksort(&mut big);
    let mut ok = true;
    for i in 1..big.len() {
        if big[i - 1] > big[i] {
            ok = false;
        }
    }
    println!("sorted {} min {} max {} mid {}", ok, big[0], big[63], big[32]);
}
