fn main() {
    let cs = ['a', 'Z', '5', ' ', '\n', '_', 'é', '9', 'f'];
    for c in cs {
        println!(
            "{:?}: alpha {} num {} alnum {} ws {} up {} low {} ascii {} digit16 {:?}",
            c,
            c.is_alphabetic(),
            c.is_numeric(),
            c.is_alphanumeric(),
            c.is_whitespace(),
            c.is_uppercase(),
            c.is_lowercase(),
            c.is_ascii(),
            c.to_digit(16)
        );
    }
    println!("{:?} {:?}", char::from_digit(7, 10), char::from_digit(11, 16));
    println!("{:?} {:?}", char::from_u32(0x41), char::from_u32(0xD800));
    println!("{}", char::from(66u8));
    println!("{}", ('a' as u8 + 2) as char);
    println!("{}", 'q'.to_ascii_uppercase());
    println!("{}", 'Q'.to_lowercase());
    println!("{}", 'ß'.to_uppercase());
    println!("{}", 'x'.is_ascii_hexdigit());
    println!("{}", '#'.is_ascii_punctuation());
    println!("{}", 'é'.len_utf8());
    println!("{}", '€'.len_utf8());
    let mut buf = [0u8; 4];
    let enc = '€'.encode_utf8(&mut buf);
    println!("{} {:?}", enc, enc.as_bytes());
    println!("{}", ('a'..='e').collect::<String>());
    println!("{:?}", ('x'..'{').collect::<Vec<char>>());
    println!("{}", 'A' < 'a');
    println!("{}", u32::from('Ω'));
    println!("{}", '7'.is_ascii_digit());
    println!("{:?}", '\t'.is_ascii_whitespace());
    println!("{}", 'Z'.eq_ignore_ascii_case(&'z'));
}
