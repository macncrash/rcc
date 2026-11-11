fn parse_int(s: &str) -> Result<i64, &'static str> {
    let b = s.as_bytes();
    if b.is_empty() {
        return Err("empty");
    }
    let (neg, digits) = match b[0] {
        b'-' => (true, &b[1..]),
        b'+' => (false, &b[1..]),
        _ => (false, b),
    };
    if digits.is_empty() {
        return Err("no digits");
    }
    let mut v: i64 = 0;
    for i in 0..digits.len() {
        let d = digits[i];
        if !(b'0'..=b'9').contains(&d) {
            return Err("bad digit");
        }
        v = v * 10 + (d - b'0') as i64;
    }
    Ok(if neg { -v } else { v })
}

fn sum_csv(line: &str) -> Result<i64, &'static str> {
    let mut total = 0;
    for field in line.split(',') {
        let f = field.trim();
        let v = parse_int(f)?;
        println!("  field [{}] = {}", f, v);
        total += v;
    }
    Ok(total)
}

fn main() {
    let lines = ["1, 2, 3", "10,-20, +30", "5, x, 7", " 100 ", "4,,5", "-"];
    for i in 0..lines.len() {
        match sum_csv(lines[i]) {
            Ok(t) => println!("line {} total {}", i, t),
            Err(e) => println!("line {} error {}", i, e),
        }
    }
    for w in "the quick  brown fox".split_whitespace() {
        print!("<{}>", w);
    }
    println!();
    for part in "a=1;b=2;c".split(';') {
        match part.split_once('=') {
            Some((k, v)) => println!("key {} val {}", k, v),
            None => println!("bare {}", part),
        }
    }
    let n: i32 = "123".parse().unwrap();
    let m: Result<u8, _> = "300".parse::<u8>();
    println!("{} {}", n + 1, m.is_err());
    let f: f64 = "2.5".parse().unwrap();
    println!("{}", f * 2.0);
    let mut i = 0;
    for l in "one\ntwo\r\nthree".lines() {
        println!("{} {}", i, l);
        i += 1;
    }
}
