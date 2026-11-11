use std::fmt;

struct Celsius(f64);
impl fmt::Display for Celsius {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.1}C", self.0)
    }
}

fn describe<T: ToString>(items: &[T]) -> String {
    items.iter().map(|i| i.to_string()).collect::<Vec<_>>().join("/")
}

fn main() {
    println!("{}", 42.to_string());
    println!("{}", (-7i64).to_string());
    println!("{}", 3.5f64.to_string());
    println!("{}", 1e20f64.to_string());
    println!("{}", 'c'.to_string());
    println!("{}", true.to_string());
    println!("{}", "str".to_string());
    println!("{}", String::from("s").to_string());
    println!("{}", Celsius(21.456).to_string());
    println!("{}", describe(&[1, 2, 3]));
    println!("{}", describe(&[Celsius(0.0), Celsius(-3.25)]));
    println!("{}", describe(&['a', 'b']));
    let n: u128 = u128::MAX;
    let s = n.to_string();
    println!("{} {}", s, s.len());
    let back: u128 = s.parse().unwrap();
    println!("{}", back == n);
    let joined: String = (1..=5).map(|i| i.to_string()).collect();
    println!("{}", joined);
    let f = format!("{}", Celsius(100.0));
    println!("{} {}", f, f.len());
    let e = "x".parse::<i32>().unwrap_err();
    println!("{}", e.to_string());
    let fe = "1.2.3".parse::<f64>().unwrap_err();
    println!("{}", fe);
    let be = "yes".parse::<bool>().unwrap_err();
    println!("{}", be);
    let ce = "ab".parse::<char>().unwrap_err();
    println!("{}", ce);
    println!("{:?}", "true".parse::<bool>());
    println!("{:?}", "z".parse::<char>());
    println!("{:?}", "2.5e3".parse::<f64>());
    println!("{:?}", "inf".parse::<f64>());
    println!("{:?}", "-nan".parse::<f64>().map(|x| x.is_nan()));
    println!("{}", 0.1f32.to_string());
    println!("{}", (1u8 as char).to_string().len());
}
