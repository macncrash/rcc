fn inc(x: &mut i32) {
    *x += 1;
}

fn swap(a: &mut i64, b: &mut i64) {
    let t = *a;
    *a = *b;
    *b = t;
}

fn fill(arr: &mut [u32; 8], v: u32) {
    for i in 0..8 {
        arr[i] = v + i as u32;
    }
}

fn bump_pair(p: &mut (i32, i32)) {
    p.0 *= 10;
    p.1 -= 1;
}

fn get_mut(arr: &mut [i32; 3], i: usize) -> &mut i32 {
    &mut arr[i]
}

fn main() {
    let mut n = 0;
    inc(&mut n);
    inc(&mut n);
    println!("n = {}", n);
    {
        let r = &mut n;
        *r *= 50;
        inc(r);
    }
    println!("n = {}", n);
    let mut a = 1;
    let mut b = 2;
    swap(&mut a, &mut b);
    println!("a={} b={}", a, b);
    let mut arr = [0u32; 8];
    fill(&mut arr, 100);
    for v in arr {
        print!("{} ", v);
    }
    println!();
    let mut p = (3, 3);
    bump_pair(&mut p);
    bump_pair(&mut p);
    println!("{} {}", p.0, p.1);
    let mut small = [1, 2, 3];
    *get_mut(&mut small, 1) = 20;
    *get_mut(&mut small, 2) += 300;
    println!("{} {} {}", small[0], small[1], small[2]);
    let mut grid = [[0u8; 3]; 3];
    for i in 0..3 {
        let row = &mut grid[i];
        for j in 0..3 {
            row[j] = (i * 3 + j) as u8;
        }
    }
    println!("{} {} {}", grid[0][2], grid[1][1], grid[2][0]);
    let mut x = 5;
    let rx = &mut x;
    let rrx = &mut *rx;
    *rrx += 1;
    *rx += 1;
    println!("x={}", x);
    let mut f = 1.5;
    let fr = &mut f;
    *fr *= 4.0;
    println!("f={}", f);
    let mut flag = false;
    let flag_ref = &mut flag;
    *flag_ref = !*flag_ref;
    println!("{}", flag);
}
