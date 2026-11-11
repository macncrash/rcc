fn main() {
    let s = String::from("héllo");
    println!("len {}", s.len());
    println!("{}", &s[0..1]);
    let k = s.len() - 4;
    println!("about to slice at {}", k);
    println!("{}", &s[0..k]);
    println!("unreachable");
}
