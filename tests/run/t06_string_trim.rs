fn main() {
    let s = "  \t hello world \n ";
    println!("[{}]", s.trim());
    println!("[{}]", s.trim_start());
    println!("[{}]", s.trim_end());
    println!("[{}]", "xxabcxx".trim_matches('x'));
    println!("[{}]", "xxabcxx".trim_start_matches('x'));
    println!("[{}]", "xxabcxx".trim_end_matches('x'));
    println!("[{}]", "123abc456".trim_matches(|c: char| c.is_ascii_digit()));
    println!("[{}]", "--a--".trim_start_matches("--"));
    println!("{:?}", "prefix_body".strip_prefix("prefix_"));
    println!("{:?}", "prefix_body".strip_prefix("nope"));
    println!("{:?}", "file.rs".strip_suffix(".rs"));
    println!("{:?}", "file.rs".strip_suffix(".c"));
    println!("{} {}", "hello".starts_with("he"), "hello".starts_with('x'));
    println!("{} {}", "hello".ends_with("llo"), "hello".ends_with(char::is_numeric));
    let args = ["-O2", "-c", "--verbose", "file.c", "-I/usr"];
    for a in args {
        if let Some(rest) = a.strip_prefix("--") {
            println!("long {}", rest);
        } else if let Some(rest) = a.strip_prefix("-I") {
            println!("include {}", rest);
        } else if let Some(rest) = a.strip_prefix('-') {
            println!("short {}", rest);
        } else {
            println!("positional {}", a);
        }
    }
    let owned = String::from("   padded   ");
    let t: &str = owned.trim();
    println!("[{}] {}", t, t.len());
    println!("[{}]", "\u{a0}nbsp\u{a0}".trim());
}
