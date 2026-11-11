static COUNT: u32 = 10;
static NAME: &str = "static string";
static TABLE: [i16; 6] = [-3, -2, -1, 1, 2, 3];
static mut COUNTER: u64 = 0;
static mut LOG: [u8; 8] = [0; 8];

fn bump(by: u64) -> u64 {
    unsafe {
        COUNTER += by;
        COUNTER
    }
}

fn record(i: usize, v: u8) {
    unsafe {
        LOG[i] = v;
    }
}

fn main() {
    println!("{}", COUNT);
    println!("{}", NAME);
    let mut s = 0;
    for i in 0..TABLE.len() {
        s += TABLE[i] * TABLE[i];
    }
    println!("sum sq = {}", s);
    for i in 0..5 {
        println!("bump -> {}", bump(i * 10));
    }
    let now = unsafe { COUNTER };
    println!("now {}", now);
    for i in 0..8 {
        record(i, (i * i) as u8);
    }
    let copy = unsafe { LOG };
    for v in copy {
        print!("{} ", v);
    }
    println!();
    static INNER: i32 = -77;
    println!("{}", INNER);
    let r: &'static i32 = &INNER;
    println!("{}", *r + 1);
}
