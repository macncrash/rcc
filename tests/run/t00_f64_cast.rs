fn main() {
    let v = 2.75f64;
    println!("{}", v as i32);
    println!("{}", -v as i32);
    println!("{}", v as u32);
    println!("{}", v as i64);
    println!("{}", (v * 1000.0) as u64);
    println!("{}", 255.9 as u8);
    println!("{}", -1.5 as i8);
    let i: i32 = 7;
    let f = i as f64 / 2.0;
    println!("{}", f);
    println!("{}", (i / 2) as f64);
    let big: i64 = 9007199254740993;
    println!("{}", big as f64);
    println!("{}", (big as f64) as i64);
    let u: u32 = 4000000000;
    println!("{}", u as f64);
    println!("{}", u as f32);
    let r = 0.5f64;
    println!("{}", (r * 3.0) as i32);
    let mut c = 0;
    while c <= 100 {
        let fahr = c as f64 * 9.0 / 5.0 + 32.0;
        println!("{}C = {}F", c, fahr);
        c += 25;
    }
    let half: f32 = 0.5;
    println!("{}", half as f64);
    println!("{}", (1.0f64 / 3.0) as f32);
    println!("{}", 16777217i32 as f32);
    println!("{}", -0.0f64);
    println!("{}", 0.0f64 == -0.0);
    println!("{}", 1e300 * 10.0);
    println!("{}", 3.0e15f64);
}
