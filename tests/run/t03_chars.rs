fn main() {
    let s = "Hello World 123 _x!";
    let mut upper = 0;
    let mut lower = 0;
    let mut digits = 0;
    let mut ws = 0;
    let mut other = 0;
    for c in s.chars() {
        if c.is_ascii_uppercase() {
            upper += 1;
        } else if c.is_ascii_lowercase() {
            lower += 1;
        } else if c.is_ascii_digit() {
            digits += 1;
        } else if c.is_whitespace() {
            ws += 1;
        } else {
            other += 1;
        }
    }
    println!("{} {} {} {} {}", upper, lower, digits, ws, other);
    for c in "aZ9 _".chars() {
        println!("{} alnum={} alpha={} upper={} lower={} punct={}",
            c as u32, c.is_alphanumeric(), c.is_ascii_alphabetic(), c.to_ascii_uppercase(),
            c.to_ascii_lowercase(), c.is_ascii_punctuation());
    }
    println!("{} {}", '7'.to_digit(10).unwrap(), 'f'.to_digit(16).unwrap());
    println!("{}", 'g'.to_digit(16).is_none());
    println!("{}", std::char::from_digit(7, 10).unwrap());
    println!("{}", char::from_u32(0x263A).unwrap());
    println!("{}", char::from_u32(0xD800).is_none());
    println!("{} {}", 'é'.len_utf8(), '→'.len_utf8());
    println!("{}", ('a' as u8 + 2) as char);
    println!("{}", char::from(66u8));
    let mut buf = [0u8; 4];
    let enc = '€'.encode_utf8(&mut buf);
    println!("{} {} {} {}", enc.len(), buf[0], buf[1], buf[2]);
    println!("{}", 'A' < 'a');
    let mut rot = 0u32;
    for c in "abcxyz".chars() {
        let r = (((c as u8 - b'a') + 13) % 26 + b'a') as char;
        print!("{}", r);
        rot += r as u32;
    }
    println!(" {}", rot);
    println!("{}", 'x'.eq_ignore_ascii_case(&'X'));
}
