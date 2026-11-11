extern "C" {
    fn putchar(c: i32) -> i32;
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn fact(n: u64) -> u64 {
    if n <= 1 { 1 } else { n * fact(n - 1) }
}

fn main() {
    let x = add(40, 2);
    let mut i = 0;
    let mut s = 0u64;
    while i < 5 {
        s += fact(i);
        i += 1;
    }
    unsafe {
        putchar(48 + (x - 40));
        putchar(48 + (s as i32) % 10);
        putchar(10);
    }
}
