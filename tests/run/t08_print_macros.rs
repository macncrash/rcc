fn main() {
    print!("a");
    print!("{}", 'b');
    println!("{}", "c");
    println!();
    eprintln!("this goes to stderr {}", 1);
    eprint!("so does this\n");
    let multi = "line one
line two";
    println!("{}", multi);
    let cont = "joined \
                with continuation";
    println!("{}", cont);
    println!("{}", r"C:\raw\path");
    println!("{}", r#"has "quotes" inside"#);
    println!("{}", concat!("con", "cat", 1, 2.5, true));
    println!("{}", stringify!(a + b * c));
    println!(concat!("from ", "concat {}"), 9);
    let s = format!("{}", 1) + &format!("{:?}", "2");
    println!("{}", s);
    println!("{}", line!() > 0);
    println!("{}", column!() > 0);
    println!("{}", file!().ends_with(".rs"));
    println!("{:?}", module_path!());
    let n = 3;
    println!("{}", if n > 2 { "big" } else { "small" });
    println!("{}{}", "x".repeat(3), "-".repeat(0));
    println!("{}", '\u{41}');
    println!("{}", "\x41\x42");
    println!("{}", b'A');
    println!("{:?}", b"\x41\n");
    println!("{}", std::str::from_utf8(b"bytes").unwrap());
}
