fn parse_digit(c: u8) -> Result<u32, &'static str> {
    if c >= b'0' && c <= b'9' {
        Ok((c - b'0') as u32)
    } else {
        Err("not a digit")
    }
}

fn main() {
    let s = "314x5";
    let b = s.as_bytes();
    let mut acc = 0;
    for i in 0..b.len() {
        let d = parse_digit(b[i]).expect("digit expected");
        acc = acc * 10 + d;
        println!("acc {}", acc);
    }
    println!("final {}", acc);
}
