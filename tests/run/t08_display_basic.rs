fn main() {
    println!("{}", "plain str");
    println!("{}", String::from("owned string"));
    println!("{} {} {}", 1u8, -2i16, 300000u32);
    println!("{} {}", true, false);
    println!("{} {} {}", 'a', 'Z', '\u{e9}');
    println!("{} {} {} {}", 1.5f64, -0.25f32, 100.0f64, 3.0f32);
    println!("{}{}{}", "no", "spaces", 7);
    println!("{{literal braces}} {} {{{}}}", 1, 2);
    println!();
    println!("{}", "tab\there");
    println!("{}", "newline\ninside");
    println!("{}", "unicode: \u{3bb} \u{1F600}");
    let r = &&&5;
    println!("deref chain {}", r);
    let b = Box::new(12);
    println!("box {}", b);
    let s: &str = "slice";
    println!("{} {}", &s[1..3], s.len());
    print!("print ");
    print!("without ");
    print!("newline\n");
    println!("{}", 0usize);
    println!("{}", isize::MIN);
    println!("{:?}", ());
}
