use std::cmp::Ordering;
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (pa, na) = split_num(a);
    let (pb, nb) = split_num(b);
    pa.cmp(pb).then(na.cmp(&nb))
}
fn split_num(s: &str) -> (&str, u64) {
    let i = s.find(|c: char| c.is_ascii_digit()).unwrap_or(s.len());
    (&s[..i], s[i..].parse().unwrap_or(0))
}
fn main() {
    let mut files = vec!["file10", "file2", "file1", "doc3", "file20", "doc12"];
    let mut lex = files.clone();
    lex.sort();
    println!("{:?}", lex);
    files.sort_by(|a, b| natural_cmp(a, b));
    println!("{:?}", files);
    println!("{:?}", "abc".cmp("abd"));
    println!("{:?}", "b".cmp("abc"));
    println!("{:?}", "".cmp("a"));
    println!("{:?}", "Z".cmp("a"));
    let mut owned: Vec<String> = vec!["delta".into(), "Alpha".into(), "charlie".into(), "Bravo".into()];
    owned.sort();
    println!("{:?}", owned);
    owned.sort_by_key(|s| s.to_lowercase());
    println!("{:?}", owned);
    owned.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    println!("{:?}", owned);
    println!("{:?}", owned.iter().max());
    println!("{:?}", owned.iter().min_by_key(|s| s.len()));
    let s1 = String::from("same");
    let s2 = "sa".to_string() + "me";
    println!("{} {:?} {}", s1 == s2, s1.partial_cmp(&s2), s1.as_str() == "same");
}
