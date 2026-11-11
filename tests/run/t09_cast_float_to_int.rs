// float -> int `as` casts saturate; NaN -> 0; truncation toward zero
fn main() {
    let vals: [f64; 16] = [0.0, -0.0, 0.9, -0.9, 1.5, -1.5, 127.9, 128.0, 255.5, 256.0, -129.0, 3e9, -3e9, 1e20, -1e20, 4294967295.5];
    for &v in vals.iter() {
        println!(
            "{}: u8 {} i8 {} u16 {} i16 {} u32 {} i32 {} u64 {} i64 {}",
            v, v as u8, v as i8, v as u16, v as i16, v as u32, v as i32, v as u64, v as i64
        );
    }
    let specials: [f64; 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
    for &v in specials.iter() {
        println!(
            "{}: u8 {} i8 {} u32 {} i32 {} u64 {} i64 {} u128 {} i128 {}",
            v, v as u8, v as i8, v as u32, v as i32, v as u64, v as i64, v as u128, v as i128
        );
    }
    let big: [f64; 4] = [1e38, 1e40, -1e40, 3.4e38];
    for &v in big.iter() {
        println!("{}: u128 {} i128 {}", v, v as u128, v as i128);
    }
    let fv: [f32; 6] = [-1.0, 1e10, -1e10, 65535.9, f32::NAN, f32::INFINITY];
    for &v in fv.iter() {
        println!("f32 {}: u8 {} i16 {} u32 {} i64 {} usize {} isize {}", v, v as u8, v as i16, v as u32, v as i64, v as usize, v as isize);
    }
    // boundary values near i64 range
    println!("9.2e18 as i64 {}", 9.2e18f64 as i64);
    println!("9.3e18 as i64 {}", 9.3e18f64 as i64);
    println!("1.8e19 as u64 {}", 1.8e19f64 as u64);
    println!("1.9e19 as u64 {}", 1.9e19f64 as u64);
    println!("-9.3e18 as i64 {}", -9.3e18f64 as i64);
    println!("2^63 as i64 {}", 9223372036854775808.0f64 as i64);
    println!("-2^63 as i64 {}", -9223372036854775808.0f64 as i64);
}
