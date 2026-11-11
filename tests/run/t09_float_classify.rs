// is_nan, is_infinite, is_finite, is_normal, is_sign_negative/positive, classify, NaN comparisons
fn main() {
    let zero = 0.0f64;
    let vals: [f64; 9] = [0.0, -0.0, 1.0, -1.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 1e-310, f64::MAX];
    for &v in vals.iter() {
        println!(
            "{}: nan {} inf {} fin {} normal {} subnormal {} neg {} pos {} {:?}",
            v,
            v.is_nan(),
            v.is_infinite(),
            v.is_finite(),
            v.is_normal(),
            v.is_subnormal(),
            v.is_sign_negative(),
            v.is_sign_positive(),
            v.classify()
        );
    }
    let n = zero / zero;
    println!("0/0 nan {}", n.is_nan());
    println!("1/0 {} -1/0 {}", 1.0 / zero, -1.0 / zero);
    println!("nan==nan {} nan<1 {} nan>1 {} nan!=nan {}", n == n, n < 1.0, n > 1.0, n != n);
    println!("-0==0 {}", -0.0f64 == 0.0);
    println!("inf-inf nan {}", (f64::INFINITY - f64::INFINITY).is_nan());
    println!("inf*0 nan {}", (f64::INFINITY * 0.0).is_nan());
    println!("max*2 {}", f64::MAX * 2.0);
    println!("min_pos/2 {}", f64::MIN_POSITIVE / 2.0);
    let fv: [f32; 4] = [f32::NAN, f32::INFINITY, -0.0, 1e-40];
    for &v in fv.iter() {
        println!("f32 {}: nan {} inf {} normal {} neg {}", v, v.is_nan(), v.is_infinite(), v.is_normal(), v.is_sign_negative());
    }
    let mut count = 0;
    let mut x = 1.0f64;
    while x.is_finite() {
        x *= 10.0;
        count += 1;
    }
    println!("overflow after {} multiplications: {}", count, x);
    let mut y = 1.0f64;
    let mut steps = 0;
    while y != 0.0 {
        y /= 2.0;
        steps += 1;
    }
    println!("underflow after {} halvings", steps);
    println!("copysign {} {}", 3.0f64.copysign(-0.0), (-3.0f64).copysign(1.0));
}
