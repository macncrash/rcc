// conversions between 128-bit integers and floats, and 64-bit ints and floats
fn main() {
    let us: [u128; 7] = [0, 1, 1 << 53, (1 << 53) + 1, 1 << 64, 1 << 100, u128::MAX];
    for &u in us.iter() {
        let f = u as f64;
        println!("u128 {} as f64 {} bits {:x} back {}", u, f, f.to_bits(), f as u128);
    }
    let is: [i128; 5] = [i128::MIN, -(1 << 80), -1, 1 << 90, i128::MAX];
    for &i in is.iter() {
        let f = i as f64;
        println!("i128 {} as f64 {} back {}", i, f, f as i128);
    }
    for &u in us.iter() {
        let f = u as f32;
        println!("u128 {} as f32 {} bits {:x}", u, f, f.to_bits());
    }
    let fs: [f64; 6] = [0.5, 1.0, 1e30, 3.4e38, 1.7e38, -5.0];
    for &f in fs.iter() {
        println!("f64 {} as u128 {} as i128 {}", f, f as u128, f as i128);
    }
    let u64s: [u64; 5] = [u64::MAX, (1 << 63) + 1, 9007199254740993, 12345678901234567, 1];
    for &u in u64s.iter() {
        let f = u as f64;
        println!("u64 {} as f64 {} bits {:x} back {}", u, f, f.to_bits(), f as u64);
    }
    let i64s: [i64; 4] = [i64::MIN, -9007199254740993, -1, i64::MAX];
    for &i in i64s.iter() {
        let f = i as f64;
        println!("i64 {} as f64 {} back {}", i, f, f as i64);
    }
    for &i in [i32::MIN, -16777217, 16777217, i32::MAX].iter() {
        let f = i as f32;
        println!("i32 {} as f32 {} as f64 {}", i, f, i as f64);
    }
    let x: f32 = 16777217.0;
    println!("f32 literal 16777217 = {}", x);
    println!("f64 -> f32 {} {} {}", 0.1f64 as f32, 1e40f64 as f32, 1e-50f64 as f32);
    println!("f32 -> f64 {}", 0.1f32 as f64);
}
