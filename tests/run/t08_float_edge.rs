fn show64(x: f64) {
    println!("{} | {:?} | {:.3} | {:e}", x, x, x, x);
}

fn show32(x: f32) {
    println!("{} | {:?} | {:.3} | {:e}", x, x, x, x);
}

fn main() {
    let vals64 = [
        0.0, -0.0, 1.0, -1.0, 0.1, 0.2, 0.1 + 0.2, 1.0 / 3.0, 2.0 / 3.0, 100.0, 1e7, 1e15, 1e16, 1e17,
        1e21, 1e22, 1e-7, 1.5e-300, 123.456, 9007199254740993.0, f64::MAX, f64::MIN, f64::MIN_POSITIVE,
        f64::EPSILON, 4.9e-324, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.30000000000000004, 1.7976931348623157e308,
    ];
    for v in vals64 {
        show64(v);
    }
    let vals32 = [
        0.0f32, -0.0, 1.0, 0.1, 0.2, 0.1 + 0.2, 1.0 / 3.0, 16777216.0, 16777217.0, 1e10, 1e-10,
        f32::MAX, f32::MIN_POSITIVE, f32::EPSILON, 1e-45, f32::NAN, f32::INFINITY, 3.4028235e38,
    ];
    for v in vals32 {
        show32(v);
    }
    println!("{} {}", 0.1f32 as f64, (0.1f64 as f32) as f64);
    println!("{} {}", 1.0e300 * 1.0e10, -1.0e300 * 1.0e10);
    println!("{} {}", (-1.0f64).sqrt(), 0.0f64 / 0.0);
    println!("{} {}", 1.0f64 / 0.0, -1.0f64 / 0.0);
    println!("{} {}", 1.0f64 / -0.0, -0.0f64 == 0.0);
    println!("{:?}", [1.0f64, 2.5, -0.0]);
    println!("{}", 2.0f64.powi(60));
    println!("{}", 2.0f64.powi(70));
    println!("{}", 2.0f64.powi(-20));
    println!("{}", 1234567.0f32);
    println!("{}", 12345678.0f32);
    println!("{}", 255u8 as f32);
    println!("{}", u64::MAX as f64);
    println!("{}", i64::MIN as f32);
}
