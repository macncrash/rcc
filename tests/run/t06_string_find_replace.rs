fn main() {
    let s = "the quick brown fox jumps over the lazy dog";
    println!("{:?}", s.find("the"));
    println!("{:?}", s.rfind("the"));
    println!("{:?}", s.find('q'));
    println!("{:?}", s.find("cat"));
    println!("{:?}", s.find(|c: char| c == 'z' || c == 'x'));
    println!("{:?}", s.rfind(char::is_whitespace));
    println!("{}", s.contains("fox"));
    println!("{}", s.contains('Z'));
    println!("{}", s.replace("the", "a"));
    println!("{}", s.replacen("the", "THE", 1));
    println!("{}", s.replace(' ', "_"));
    println!("{}", "aaaa".replace("aa", "b"));
    let idx: Vec<(usize, &str)> = s.match_indices("o").collect();
    println!("{:?}", idx);
    println!("count o {}", s.matches('o').count());
    if let Some(p) = s.find("brown") {
        println!("before [{}] after [{}]", &s[..p], &s[p + 5..]);
    }
    let path = "/usr/local/lib/libfoo.so.1";
    let base = match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    };
    println!("{}", base);
    let stem = &base[..base.find('.').unwrap_or(base.len())];
    println!("{}", stem);
    let mut t = String::from("x = 1; y = 2;");
    t = t.replace(";", ";\n");
    print!("{}", t);
    println!("{}", "tab\there".replace('\t', "    "));
    println!("{}", "AbC".to_lowercase() + &"AbC".to_uppercase());
    println!("{}", "ß".to_uppercase());
    println!("{}", "abc".eq_ignore_ascii_case("ABC"));
    let mut m = String::from("Mixed");
    m.make_ascii_uppercase();
    println!("{}", m);
    println!("{}", "x".to_ascii_lowercase());
}
