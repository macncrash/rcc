fn sum(a: [i32; 5]) -> i32 {
    let mut s = 0;
    let mut i = 0;
    while i < a.len() {
        s += a[i];
        i += 1;
    }
    s
}

fn doubled(a: [i32; 5]) -> [i32; 5] {
    let mut b = a;
    for i in 0..5 {
        b[i] *= 2;
    }
    b
}

fn main() {
    let a = [3, 1, 4, 1, 5];
    println!("len = {}", a.len());
    println!("a[0] = {} a[4] = {}", a[0], a[4]);
    println!("sum = {}", sum(a));
    let d = doubled(a);
    for i in 0..d.len() {
        println!("d[{}] = {}", i, d[i]);
    }
    println!("a still {}", a[2]);
    let zeros = [0u8; 16];
    println!("zeros len {} last {}", zeros.len(), zeros[15]);
    let mut m = [7i64; 4];
    m[1] = -1;
    m[3] += 100;
    for x in m {
        println!("m {}", x);
    }
    let idx = 2;
    println!("{}", m[idx]);
    let chars = ['h', 'e', 'l', 'l', 'o'];
    for c in chars {
        print!("{}", c);
    }
    println!();
    let bools = [true, false, true];
    println!("{} {} {}", bools[0], bools[1], bools[2]);
    let empty: [u32; 0] = [];
    println!("empty len {}", empty.len());
    let eq = a == [3, 1, 4, 1, 5];
    let ne = a != d;
    println!("{} {}", eq, ne);
    let big = [1u64; 1000];
    let mut s = 0;
    for x in big {
        s += x;
    }
    println!("big sum {}", s);
    let floats = [0.5, 1.5, 2.5];
    println!("{}", floats[0] + floats[1] + floats[2]);
}
