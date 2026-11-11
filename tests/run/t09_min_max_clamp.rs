// Ord::min/max/clamp on ints, f64::min/max/clamp, std::cmp functions
use std::cmp;

fn main() {
    let xs: [i32; 6] = [-20, -5, 0, 5, 10, 99];
    for &x in xs.iter() {
        println!("{}: min3 {} max3 {} clamp(-5,10) {}", x, x.min(3), x.max(3), x.clamp(-5, 10));
    }
    println!("cmp::min {} cmp::max {}", cmp::min(4u8, 9u8), cmp::max(-4i64, -9i64));
    println!("u64 clamp {}", 1000u64.clamp(1, 100));
    let fs: [f64; 5] = [-1.5, 0.0, 0.25, 7.0, 1e10];
    for &f in fs.iter() {
        println!("{}: min {} max {} clamp {}", f, f.min(0.5), f.max(0.5), f.clamp(-1.0, 1.0));
    }
    let nan = f64::NAN;
    println!("nan.min(1) {} 1.min(nan) {} nan.max(2) {}", nan.min(1.0), 1.0f64.min(nan), nan.max(2.0));
    println!("nan clamp is_nan {}", nan.clamp(0.0, 1.0).is_nan());
    println!("f32 min {} max {}", 1.5f32.min(-2.5), 1.5f32.max(-2.5));
    let mut lo = i32::MAX;
    let mut hi = i32::MIN;
    let data = [17, -3, 44, 0, -91, 12, 8];
    for &d in data.iter() {
        lo = lo.min(d);
        hi = hi.max(d);
        println!("after {} lo {} hi {}", d, lo, hi);
    }
    println!("ordering {:?} {:?} {:?}", 1.cmp(&2), 2.cmp(&2), 3.cmp(&2));
    println!("partial {:?} {:?}", 1.0f64.partial_cmp(&2.0), nan.partial_cmp(&1.0));
    println!("total_cmp {:?} {:?}", (-0.0f64).total_cmp(&0.0), nan.total_cmp(&f64::INFINITY));
    println!("max_by_key-ish {}", cmp::max_by_key(-7i32, 5i32, |v: &i32| v.abs()));
}
