const MAX: u32 = 100;
const NEG: i64 = -42;
const PI_ISH: f64 = 3.14159;
const GREETING: &str = "hello const";
const SQUARES: [u32; 5] = [0, 1, 4, 9, 16];
const DERIVED: u32 = MAX * 2 + 1;
const SHIFTED: u64 = 1 << 40;
const MASK: u32 = !0 >> 4;
const PAIR: (i32, char) = (7, 'k');
const FLAG: bool = MAX > 50 && NEG < 0;
const BIG: u128 = u128::MAX / 2;

const fn triple(x: u32) -> u32 {
    x * 3
}

const TRIPLED: u32 = triple(14);

const fn fib(n: u32) -> u64 {
    let mut a = 0;
    let mut b = 1;
    let mut i = 0;
    while i < n {
        let t = a + b;
        a = b;
        b = t;
        i += 1;
    }
    a
}

const FIB50: u64 = fib(50);

fn main() {
    println!("{}", MAX);
    println!("{}", NEG);
    println!("{}", PI_ISH);
    println!("{}", GREETING);
    for i in 0..SQUARES.len() {
        print!("{} ", SQUARES[i]);
    }
    println!();
    println!("{}", DERIVED);
    println!("{}", SHIFTED);
    println!("{}", MASK);
    println!("{} {}", PAIR.0, PAIR.1);
    println!("{}", FLAG);
    println!("{}", BIG);
    println!("{}", TRIPLED);
    println!("{}", FIB50);
    const LOCAL: i32 = -5;
    println!("{}", LOCAL * 2);
    let arr = [0u8; MAX as usize];
    println!("{}", arr.len());
    println!("{}", triple(LOCAL as u32 & 0xFF));
    let mut total = 0;
    for i in 0..MAX {
        total += i;
    }
    println!("{}", total);
}
