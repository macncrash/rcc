// to_bits / from_bits for f64 and f32, plus special constants
fn main() {
    let vals: [f64; 12] = [0.0, -0.0, 1.0, -1.0, 0.5, 0.1, 3.141592653589793, 1e300, 1e-300, 5e-324, f64::MAX, f64::MIN_POSITIVE];
    for &v in vals.iter() {
        println!("f64 {} bits {:016x}", v, v.to_bits());
    }
    println!("inf {:x} -inf {:x}", f64::INFINITY.to_bits(), f64::NEG_INFINITY.to_bits());
    println!("nan is_nan {}", f64::NAN.is_nan());
    println!("eps {:x}", f64::EPSILON.to_bits());
    let fvals: [f32; 8] = [0.0, -0.0, 1.0, 0.1, -2.5, f32::MAX, f32::MIN_POSITIVE, 1e-45];
    for &v in fvals.iter() {
        println!("f32 {} bits {:08x}", v, v.to_bits());
    }
    let raw: [u64; 6] = [0x3ff0000000000000, 0x4000000000000000, 0xc010000000000000, 0x0000000000000001, 0x7fefffffffffffff, 0x3fb999999999999a];
    for &r in raw.iter() {
        let f = f64::from_bits(r);
        println!("from_bits {:x} = {} back {:x}", r, f, f.to_bits());
    }
    let rawf: [u32; 4] = [0x3f800000, 0x40490fdb, 0xbf000000, 0x00000001];
    for &r in rawf.iter() {
        println!("f32 from_bits {:x} = {}", r, f32::from_bits(r));
    }
    // decompose a double into sign, exponent, mantissa
    for &v in [1.0f64, -6.25, 1024.0, 0.1].iter() {
        let b = v.to_bits();
        let sign = b >> 63;
        let exp = ((b >> 52) & 0x7ff) as i64 - 1023;
        let mant = b & ((1u64 << 52) - 1);
        println!("{} sign {} exp {} mant {:x}", v, sign, exp, mant);
    }
    // next representable values via bit increment
    let one = 1.0f64.to_bits();
    println!("next after 1 {}", f64::from_bits(one + 1));
    println!("prev before 1 {}", f64::from_bits(one - 1));
    println!("0.1+0.2 bits {:x}", (0.1f64 + 0.2).to_bits());
    println!("f32 0.1+0.2 bits {:x}", (0.1f32 + 0.2).to_bits());
}
