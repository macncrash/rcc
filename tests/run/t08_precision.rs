fn main() {
    let pi = 3.14159265358979f64;
    for p in 0..8 {
        println!("{:.*}", p, pi);
    }
    println!("{:.0} {:.0} {:.0} {:.0} {:.0}", 0.5, 1.5, 2.5, -0.5, 3.5);
    println!("{:.1} {:.1} {:.1} {:.1}", 0.05, 0.15, 0.25, 0.35);
    println!("{:.2} {:.2} {:.3}", 1.005, 2.675, 1.0005);
    println!("{:.3} {:.3}", 1e-10, 123456789.0);
    println!("{:.20}", 0.1);
    println!("{:.30}", 1.0 / 3.0);
    println!("{:.5}", 1e21);
    println!("[{:10.3}] [{:<10.3}] [{:^10.3}]", pi, pi, pi);
    println!("[{:.3}] [{:.0}] [{:.10}]", "abcdef", "abc", "short");
    println!("[{:8.3}] [{:>8.3}] [{:^8.2}]", "abcdef", "abcdef", "abcdef");
    println!("[{:.2}]", "\u{3bb}\u{3bc}\u{3bd}");
    let prec = 4;
    let width = 12;
    println!("[{:width$.prec$}] [{:>1$.2$}]", pi, width, prec, width = width, prec = prec);
    println!("[{:.prec$}] [{:.*}]", 2.0f32 / 3.0, 1, 9.95f32, prec = prec);
    println!("{:.3?} {:.1?}", vec![1.0, 2.25], (0.25, 1.75));
    println!("{:.2} {:.2}", f64::NAN, f64::INFINITY);
    println!("{:.3} {:.3}", 0.0f32, -0.0f64);
    println!("{:.4} {:.4}", 1u32 as f64 / 7.0, 22.0f32 / 7.0);
    println!("{:.15}", 0.1f32);
    println!("{:.2}", 9.999);
    println!("{:.1}", 1e15 + 0.3);
}
