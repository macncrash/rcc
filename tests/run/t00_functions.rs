fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn mul3(a: i64, b: i64, c: i64) -> i64 {
    return a * b * c;
}

fn early(n: i32) -> i32 {
    if n < 0 {
        return 0;
    }
    n * 2
}

fn noret(x: i32) {
    println!("noret got {}", x);
}

fn unit_ret() -> () {
    println!("unit");
}

fn square(x: u32) -> u32 {
    x * x
}

fn sum_squares(a: u32, b: u32) -> u32 {
    square(a) + square(b)
}

fn is_even(n: u32) -> bool {
    n % 2 == 0
}

fn many(a: u8, b: u16, c: u32, d: u64, e: i8, f: i16, g: i32, h: i64) -> i64 {
    a as i64 + b as i64 + c as i64 + d as i64 + e as i64 + f as i64 + g as i64 + h as i64
}

fn main() {
    println!("{}", add(2, 3));
    println!("{}", add(add(1, 2), add(3, 4)));
    println!("{}", mul3(10, 20, 30));
    println!("{}", early(-5));
    println!("{}", early(21));
    noret(77);
    unit_ret();
    println!("{}", sum_squares(3, 4));
    println!("{}", is_even(10));
    println!("{}", is_even(7));
    println!("{}", many(1, 2, 3, 4, -5, -6, -7, -8));
    let f = add(100, -1);
    println!("{}", f);
    fn inner(x: i32) -> i32 {
        x + 1000
    }
    println!("{}", inner(5));
}
