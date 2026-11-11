fn main() {
    println!("first line");
    println!("");
    println!("tab\tseparated\tvalues");
    println!("quote \" and backslash \\");
    println!("braces {{ and }} escaped");
    println!("unicode: caf\u{e9} \u{3bb}");
    println!("multi
line literal");
    println!("continued \
              line");
    println!(r"raw \n string");
    println!(r#"raw with "quotes""#);
    println!("{}", 0);
    println!("{}", 1);
    println!("{}", -1);
    println!("{} {} {}", 1, 22, 333);
    println!("a{}b{}c", 7, 8);
    println!("{}", "str arg");
    println!();
    println!("end");
}
