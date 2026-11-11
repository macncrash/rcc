fn sum_range(lo: i32, hi: i32) -> i32 {
    let mut s = 0;
    for i in lo..hi {
        s += i;
    }
    s
}

fn main() {
    println!("{}", sum_range(0, 10));
    println!("{}", sum_range(-10, 10));
    println!("{}", sum_range(5, 0));
    let r = 0..10;
    println!("{} {} {}", r.start, r.end, r.len());
    println!("{}", (0..10).contains(&5));
    println!("{}", (0..10).contains(&10));
    println!("{}", (0..=10).contains(&10));
    let inc = 1..=3;
    println!("{} {}", inc.start(), inc.end());
    let mut count = 0u64;
    for _ in 0u64..1_000_000 {
        count += 1;
    }
    println!("{}", count);
    for i in (0..10).rev() {
        if i % 4 == 0 {
            print!("{} ", i);
        }
    }
    println!();
    let mut x = 0usize;
    for i in 0..=255u8 {
        x += i as usize;
    }
    println!("{}", x);
    for i in i8::MIN..=-125 {
        print!("{} ", i);
    }
    println!();
    for i in (u32::MAX - 2)..=u32::MAX {
        print!("{} ", i);
    }
    println!();
    let arr = [5, 6, 7, 8, 9];
    let mut s = 0;
    for i in 1..arr.len() - 1 {
        s += arr[i];
    }
    println!("{}", s);
}
