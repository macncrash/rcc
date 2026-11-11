fn main() {
    let s = "abc";
    println!("[{:10}]", s);
    println!("[{:<10}]", s);
    println!("[{:>10}]", s);
    println!("[{:^10}]", s);
    println!("[{:^9}]", s);
    println!("[{:*^10}]", s);
    println!("[{:-<8}]", s);
    println!("[{:#>8}]", s);
    println!("[{:2}]", "longer than width");
    println!("[{:10}]", 42);
    println!("[{:<10}]", 42);
    println!("[{:^10}]", 42);
    println!("[{:>10}]", -42);
    println!("[{:_^11}]", -42);
    println!("[{:10}]", 'c');
    println!("[{:>5}]", true);
    println!("[{:8}]", 1.5);
    println!("[{:<8}]", 1.5);
    println!("[{:^8.2}]", 1.5);
    println!("[{:\u{2500}^9}]", "ok");
    println!("[{:^6}]", "\u{3bb}\u{3bb}");
    for w in 0..6 {
        println!("[{:>w$}] [{:<w$}] [{:^w$}]", "x", 7, "yz", w = w);
    }
    for (i, name) in ["main", "f", "helper_fn"].iter().enumerate() {
        println!("{:>3} | {:<12}| {:>6}", i, name, name.len() * 100);
    }
    println!("[{:5}] [{:<5}] [{:^5}]", Some(1).unwrap(), "", "");
    println!("[{:>6?}] [{:<8?}]", "q", Some(1));
    println!("[{:5?}]", vec![1, 22]);
}
