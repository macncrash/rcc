fn main() {
    let a = 5;
    let b = "five";
    let s = format!("{} is {}", a, b);
    println!("{} len {}", s, s.len());
    let t = format!("{a}-{b}-{}", a * 2);
    println!("{}", t);
    let mut acc = String::new();
    for i in 0..5 {
        acc = format!("{}{}", acc, i);
        acc += &format!("[{}]", i * i);
    }
    println!("{}", acc);
    let v = vec![1, 2, 3];
    let d = format!("{:?}", v);
    println!("{} {}", d, d.len());
    let n = 255.to_string();
    println!("{} {}", n, n.len());
    println!("{}", (-17i64).to_string() + &3.5f64.to_string());
    println!("{}", 'c'.to_string() + &true.to_string());
    let pieces: Vec<String> = (1..4).map(|i| format!("<{}>", i)).collect();
    println!("{}", pieces.concat());
    let nested = format!("{}", format!("{}", format!("deep{}", 1)));
    println!("{}", nested);
    let pad = format!("{:>5}|{:<5}|{:^5}|", "ab", "cd", "ef");
    println!("{}", pad);
}
