// Shortest round-trip float printing on computed values.
fn main() {
    let mut x = 1.0f64;
    for i in 0..25 {
        x = x * 1.7 + 0.1 / (i as f64 + 1.0);
        let s = x.to_string();
        let back: f64 = s.parse().unwrap();
        println!("{:2} {} {:?} rt={}", i, s, x / 1e6, back == x);
    }
    let mut y = 1.0f32;
    for i in 0..15 {
        y = y / 3.0 + i as f32;
        let back: f32 = y.to_string().parse().unwrap();
        println!("f32 {} {} rt={}", y, y as f64, back == y);
    }
    let mut h = 0.5f64;
    for _ in 0..12 {
        h /= 10.0;
        print!("{} ", h);
    }
    println!();
    let mut g = 3.0f64;
    for _ in 0..8 {
        g *= 1000.0;
        print!("{} ", g);
    }
    println!();
    let sums: Vec<f64> = (1..=10).map(|n| (1..=n).map(|k| 1.0 / k as f64).sum()).collect();
    println!("{:?}", sums);
    let sq: Vec<f64> = (1..10).map(|n| (n as f64).sqrt()).collect();
    println!("{:?}", sq);
    for s in ["0.1", "1e-7", "123.456e2", "-0", "1.0000000000000002", "9007199254740993", "2.2250738585072014e-308"] {
        let v: f64 = s.parse().unwrap();
        println!("{} -> {} / {:?}", s, v, v);
    }
}
