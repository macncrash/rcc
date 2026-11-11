fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn indent(text: &str, n: usize) -> String {
    let pad = " ".repeat(n);
    text.lines().map(|l| if l.is_empty() { String::new() } else { format!("{}{}", pad, l) }).collect::<Vec<_>>().join("\n")
}
fn camel_to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
fn main() {
    println!("{}", escape("say \"hi\"\n\tback\\slash\u{1}"));
    println!("{}", indent("fn main() {\n    x();\n\n}", 4));
    for w in ["CamelCase", "HTTPServer", "simple", "aB"] {
        println!("{} -> {}", w, camel_to_snake(w));
    }
    let mut sb = String::new();
    for i in 0..4 {
        if !sb.is_empty() {
            sb.push_str(", ");
        }
        sb += &i.to_string();
    }
    println!("{}", sb);
    let rev_words: String = "one two three".split(' ').rev().collect::<Vec<_>>().join(" ");
    println!("{}", rev_words);
    let caesar: String = "Hello".chars().map(|c| if c.is_ascii_lowercase() { (((c as u8 - b'a' + 3) % 26) + b'a') as char } else { c }).collect();
    println!("{}", caesar);
    let cap: String = "word".chars().enumerate().map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c }).collect();
    println!("{}", cap);
    let mut s = String::from("abc");
    let taken = std::mem::take(&mut s);
    println!("[{}] [{}]", s, taken);
}
