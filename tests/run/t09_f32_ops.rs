// f32 arithmetic is done in single precision (not widened)
fn main() {
    let a: f32 = 16777216.0;
    println!("{} + 1 = {}", a, a + 1.0);
    println!("{} + 2 = {}", a, a + 2.0);
    let b: f32 = 0.1;
    let mut s: f32 = 0.0;
    for _ in 0..10 {
        s += b;
    }
    println!("0.1*10 sum f32 = {} bits {:x}", s, s.to_bits());
    let mut d: f64 = 0.0;
    for _ in 0..10 {
        d += 0.1;
    }
    println!("0.1*10 sum f64 = {} bits {:x}", d, d.to_bits());
    let x: f32 = 1.0 / 3.0;
    println!("1/3 f32 {} bits {:x} as f64 {}", x, x.to_bits(), x as f64);
    println!("f32 sqrt2 {} bits {:x}", 2.0f32.sqrt(), 2.0f32.sqrt().to_bits());
    println!("f32 consts {} {} {}", f32::MAX, f32::MIN, f32::EPSILON);
    println!("f32 mul overflow {}", f32::MAX * 2.0);
    let vals: [f32; 6] = [1.5, -2.25, 1e-7, 123456.7, 3.0e38, 7.0];
    for &v in vals.iter() {
        println!("{}: *3 {} /3 {} -1 {} sq {} floor {} round {}", v, v * 3.0, v / 3.0, v - 1.0, v * v, v.floor(), v.round());
    }
    // f64 -> f32 rounding
    let ds: [f64; 5] = [0.1, 1.0000000596046448, 1.0000000894069672, 3.4028235677973366e38, 1e-46];
    for &v in ds.iter() {
        let f = v as f32;
        println!("{} -> {} bits {:x}", v, f, f.to_bits());
    }
    let mut prod: f32 = 1.0;
    for i in 1..=35 {
        prod *= i as f32;
    }
    println!("35! f32 = {} is_inf {}", prod, prod.is_infinite());
    let mut halv: f32 = 1.0;
    let mut n = 0;
    while halv > 0.0 {
        halv /= 2.0;
        n += 1;
    }
    println!("f32 underflow after {}", n);
    println!("f32 cmp {} {} {}", 0.1f32 as f64 == 0.1f64, 0.5f32 as f64 == 0.5, (0.1f32 + 0.2f32) == 0.3f32);
    println!("f32 to int {} {} {}", 2.9f32 as i32, -2.9f32 as i32, 3e9f32 as i32);
}
