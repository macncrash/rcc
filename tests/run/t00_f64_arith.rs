fn main() {
    let a: f64 = 1.5;
    let b: f64 = 0.25;
    println!("{}", a + b);
    println!("{}", a - b);
    println!("{}", a * b);
    println!("{}", a / b);
    println!("{}", a % b);
    println!("{}", -a);
    println!("{}", 0.1 + 0.2);
    println!("{}", 1.0 / 3.0);
    println!("{}", 2.0 / 3.0);
    println!("{}", 10.0);
    println!("{}", 100.0 / 4.0);
    println!("{}", 1e10);
    println!("{}", 1.5e-5);
    println!("{}", 123456.789);
    println!("{}", 7.0 % 2.5);
    println!("{}", -7.5 % 2.0);
    let mut x = 1.0;
    let mut i = 0;
    while i < 10 {
        x = x * 1.5 + 0.125;
        println!("x{} = {}", i, x);
        i += 1;
    }
    // Newton's method for sqrt(2)
    let mut g: f64 = 1.0;
    let mut k = 0;
    while k < 6 {
        g = (g + 2.0 / g) / 2.0;
        println!("newton {} = {}", k, g);
        k += 1;
    }
    let mut sum = 0.0;
    let mut n = 1;
    while n <= 1000 {
        sum += 1.0 / (n as f64 * n as f64);
        n += 1;
    }
    println!("basel partial = {}", sum);
    let c: f32 = 1.1;
    let d: f32 = 2.2;
    println!("{}", c + d);
    println!("{}", c * d);
    println!("{}", c as f64);
    println!("{}", 0.1f64 as f32);
}
