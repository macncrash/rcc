fn bubble_sort(a: &mut [i32; 10]) {
    let n = a.len();
    for i in 0..n {
        let mut swapped = false;
        for j in 0..n - 1 - i {
            if a[j] > a[j + 1] {
                let t = a[j];
                a[j] = a[j + 1];
                a[j + 1] = t;
                swapped = true;
            }
        }
        if !swapped {
            break;
        }
    }
}

fn binary_search(a: &[i32; 10], target: i32) -> i32 {
    let mut lo = 0i32;
    let mut hi = a.len() as i32 - 1;
    while lo <= hi {
        let mid = (lo + hi) / 2;
        if a[mid as usize] == target {
            return mid;
        } else if a[mid as usize] < target {
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }
    -1
}

fn reverse(a: &mut [i32; 10]) {
    let mut i = 0;
    let mut j = a.len() - 1;
    while i < j {
        let t = a[i];
        a[i] = a[j];
        a[j] = t;
        i += 1;
        j -= 1;
    }
}

fn print(a: &[i32; 10]) {
    for x in a {
        print!("{} ", x);
    }
    println!();
}

fn main() {
    let mut a = [38, -5, 12, 99, 0, 7, -40, 12, 55, 3];
    print(&a);
    bubble_sort(&mut a);
    print(&a);
    for t in [-40, 12, 99, 100, 4] {
        println!("search {} -> {}", t, binary_search(&a, t));
    }
    reverse(&mut a);
    print(&a);
    let mut max = a[0];
    let mut argmax = 0;
    for i in 1..a.len() {
        if a[i] > max {
            max = a[i];
            argmax = i;
        }
    }
    println!("max {} at {}", max, argmax);
    let mut prefix = [0i32; 10];
    let mut acc = 0;
    for i in 0..10 {
        acc += a[i];
        prefix[i] = acc;
    }
    print(&prefix);
    // selection sort descending
    let mut b = [5, 2, 8, 1, 9, 3, 7, 4, 6, 0];
    for i in 0..10 {
        let mut best = i;
        for j in i + 1..10 {
            if b[j] > b[best] {
                best = j;
            }
        }
        b.swap(i, best);
    }
    print(&b);
}
