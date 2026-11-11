// sqrt, powi, powf, abs, mul_add, hypot (exactly representable results), arithmetic
fn main() {
    for v in [0.0f64, 1.0, 2.0, 4.0, 9.0, 0.25, 1e100, 2.25] {
        println!("sqrt {} = {} bits {:x}", v, v.sqrt(), v.sqrt().to_bits());
    }
    println!("sqrt(-1) nan {}", (-1.0f64).sqrt().is_nan());
    for e in [-3i32, -1, 0, 1, 2, 10, 30] {
        println!("2.powi({}) = {} 1.5.powi(|{}|) = {}", e, 2.0f64.powi(e), e, 1.5f64.powi(e.abs()));
    }
    println!("powf {} {} {} {}", 4.0f64.powf(0.5), 2.0f64.powf(10.0), 8.0f64.powf(1.0), 1.0f64.powf(123.4));
    println!("powf zero {}", 0.0f64.powf(0.0));
    println!("abs {} {} {}", (-3.5f64).abs(), 3.5f64.abs(), (-0.0f64).abs());
    println!("mul_add {}", 2.0f64.mul_add(3.0, 4.0));
    println!("hypot {}", 3.0f64.hypot(4.0));
    println!("recip {} to_degrees(pi) {}", 4.0f64.recip(), std::f64::consts::PI.to_degrees());
    println!("consts pi {:x} e {:x}", std::f64::consts::PI.to_bits(), std::f64::consts::E.to_bits());
    // basic arithmetic precision checks
    let a = 0.1f64;
    let b = 0.2f64;
    println!("0.1+0.2 = {} = 0.3? {}", a + b, a + b == 0.3);
    println!("1/3 = {}", 1.0f64 / 3.0);
    println!("2/3 f32 = {}", 2.0f32 / 3.0);
    let mut s = 0.0f64;
    for i in 1..=100 {
        s += 1.0 / (i as f64);
    }
    println!("harmonic 100 = {} bits {:x}", s, s.to_bits());
    let mut s32 = 0.0f32;
    for i in 1..=100 {
        s32 += 1.0 / (i as f32);
    }
    println!("harmonic f32 100 = {} bits {:x}", s32, s32.to_bits());
    // Newton's method for sqrt(2)
    let mut x = 1.0f64;
    for i in 0..6 {
        x = (x + 2.0 / x) / 2.0;
        println!("newton {} {}", i, x);
    }
    println!("matches sqrt {}", x == 2.0f64.sqrt());
    println!("neg {} sub {} rem {}", -x, 10.0 - x, 10.0 % x);
}
