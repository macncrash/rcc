fn count_char(s: &str, c: char) -> u32 {
    let mut n = 0;
    for ch in s.chars() {
        if ch == c {
            n += 1;
        }
    }
    n
}

fn main() {
    let s = "hello, world";
    println!("{} len {}", s, s.len());
    println!("{}", s.is_empty());
    println!("{}", "".is_empty());
    let b = s.as_bytes();
    println!("bytes {} {} {}", b[0], b[1], b[b.len() - 1]);
    println!("starts {} {}", s.starts_with("hello"), s.starts_with("world"));
    println!("ends {} {}", s.ends_with("world"), s.ends_with('d'));
    println!("contains {} {}", s.contains("lo, w"), s.contains('z'));
    println!("l count {}", count_char(s, 'l'));
    println!("sub '{}' '{}' '{}'", &s[0..5], &s[7..], &s[..2]);
    let mut n = 0;
    for c in s.chars() {
        if c.is_alphabetic() {
            n += 1;
        }
    }
    println!("letters {}", n);
    let uni = "héllo→";
    println!("uni bytes {}", uni.len());
    let mut cn = 0;
    for c in uni.chars() {
        print!("{}:{} ", c, c as u32);
        cn += 1;
    }
    println!();
    println!("chars {}", cn);
    match s.find(',') {
        Some(i) => println!("comma at {}", i),
        None => println!("no comma"),
    }
    match s.find("xyz") {
        Some(i) => println!("found at {}", i),
        None => println!("not found"),
    }
    let t = "  padded\t";
    println!("[{}]", t.trim());
    println!("[{}] [{}]", t.trim_start(), t.trim_end());
    if let Some(rest) = s.strip_prefix("hello") {
        println!("rest [{}]", rest);
    }
    println!("{}", s == "hello, world");
    println!("{}", "abc" < "abd");
    let first = s.chars().next();
    match first {
        Some(c) => println!("first {}", c),
        None => println!("none"),
    }
    let raw = r"a\nb";
    println!("{} {}", raw, raw.len());
    let esc = "tab\there\nnewline \"quoted\" \\ \u{41}";
    println!("{}", esc);
}
