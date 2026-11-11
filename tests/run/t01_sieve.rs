const LIMIT: usize = 1000;

fn main() {
    let mut composite = [false; LIMIT + 1];
    composite[0] = true;
    composite[1] = true;
    let mut i = 2;
    while i * i <= LIMIT {
        if !composite[i] {
            let mut j = i * i;
            while j <= LIMIT {
                composite[j] = true;
                j += i;
            }
        }
        i += 1;
    }
    let mut count = 0;
    let mut last = 0;
    for n in 0..=LIMIT {
        if !composite[n] {
            count += 1;
            last = n;
            if n < 60 {
                print!("{} ", n);
            }
        }
    }
    println!();
    println!("count {} last {}", count, last);
    let mut twins = 0;
    for n in 2..LIMIT - 1 {
        if !composite[n] && !composite[n + 2] {
            twins += 1;
        }
    }
    println!("twin pairs {}", twins);
    let mut gaps = [0u32; 32];
    let mut prev = 2;
    for n in 3..=LIMIT {
        if !composite[n] {
            let g = n - prev;
            gaps[g] += 1;
            prev = n;
        }
    }
    for g in 0..32 {
        if gaps[g] > 0 {
            println!("gap {}: {}", g, gaps[g]);
        }
    }
}
