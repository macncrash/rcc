fn main() {
    let xs = [0.0, 1.0, -1.0, 1500.0, 0.00025, 6.02214076e23, 1.602e-19, 123456789.0, f64::MAX, f64::MIN_POSITIVE, 5e-324];
    for x in xs {
        println!("{:e} {:E} {:.2e} {:12.3e}|", x, x, x, x);
    }
    println!("{:e} {:e} {:e}", 1u32, 1234567u64, -500i32);
    println!("{:e} {:e}", 100u8, 0i64);
    println!("{:.0e} {:.1e} {:.3e}", 15.0, 15.5, 2.5);
    println!("{:e} {:e}", f64::NAN, f64::NEG_INFINITY);
    println!("{:e} {:e}", 1.0f32 / 3.0, 3.4e38f32);
    println!("{:+e} {:010.2e}", 42.0, -42.0);
    println!("{:?} {:?}", 1e15, 1e16);
    println!("{:?} {:?}", 1e-4, 1e-5);
    println!("{} {}", 1e15, 1e16);
    println!("{} {}", 1e-4, 1e-5);
}
