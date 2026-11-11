fn first_word(s: &str) -> &str {
    match s.find(' ') {
        Some(i) => &s[..i],
        None => s,
    }
}
fn main() {
    let s = String::from("hello world program");
    println!("{}", &s[0..5]);
    println!("{}", &s[6..]);
    println!("{}", &s[..]);
    println!("{}", first_word(&s));
    println!("{}", first_word("single"));
    println!("{:?}", s.get(0..3));
    println!("{:?}", s.get(100..));
    let u = "αβγ";
    println!("{:?}", u.get(0..1));
    println!("{:?}", u.get(0..2));
    println!("{}", &u[2..]);
    let (a, b) = s.split_at(5);
    println!("[{}] [{}]", a, b);
    let mut owned = s.clone();
    owned.replace_range(0..5, "HOWDY");
    println!("{}", owned);
    let t: String = s.chars().skip(6).take(5).collect();
    println!("{}", t);
    let bytes = s.as_bytes();
    println!("{} {}", bytes[0], bytes.len());
    let sub = &s[s.len() - 3..];
    println!("{}", sub);
    let mut parts = Vec::new();
    let mut rest: &str = &s;
    while let Some(i) = rest.find(' ') {
        parts.push(&rest[..i]);
        rest = &rest[i + 1..];
    }
    parts.push(rest);
    println!("{:?}", parts);
}
