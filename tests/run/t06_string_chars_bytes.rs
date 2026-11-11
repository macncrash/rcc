fn main() {
    let s = "héllo, wörld";
    println!("len {} chars {}", s.len(), s.chars().count());
    for c in s.chars().take(5) {
        println!("char {} code {} len_utf8 {}", c, c as u32, c.len_utf8());
    }
    let bytes: Vec<u8> = s.bytes().take(4).collect();
    println!("{:?}", bytes);
    for (i, c) in s.char_indices().filter(|(_, c)| !c.is_ascii()) {
        println!("non-ascii {} at {}", c, i);
    }
    let rev: String = s.chars().rev().collect();
    println!("{}", rev);
    println!("as_bytes[0] {} last {:?}", s.as_bytes()[0], s.chars().last());
    println!("nth {:?}", s.chars().nth(1));
    let caps: String = s.chars().map(|c| c.to_ascii_uppercase()).collect();
    println!("{}", caps);
    let ascii = "Rust 2021!";
    let mut letters = 0;
    let mut digits = 0;
    let mut other = 0;
    for b in ascii.bytes() {
        if b.is_ascii_alphabetic() {
            letters += 1;
        } else if b.is_ascii_digit() {
            digits += 1;
        } else {
            other += 1;
        }
    }
    println!("{} {} {}", letters, digits, other);
    println!("is_char_boundary {} {}", s.is_char_boundary(1), s.is_char_boundary(2));
    let v: Vec<char> = "abc".chars().collect();
    let back: String = v.iter().collect();
    println!("{:?} {}", v, back);
    let from_bytes = String::from_utf8(vec![82, 117, 115, 116]).unwrap();
    println!("{}", from_bytes);
    let bad = String::from_utf8(vec![0xff, 0x41]);
    println!("bad is_err {}", bad.is_err());
    let lossy = String::from_utf8_lossy(&[72, 105, 0xff]);
    println!("{}", lossy);
    let st = std::str::from_utf8(b"slice").unwrap();
    println!("{}", st);
    let ub = "abc".to_string().into_bytes();
    println!("{:?}", ub);
}
