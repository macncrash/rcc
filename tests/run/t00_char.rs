fn main() {
    let a = 'a';
    let z = 'Z';
    let digit = '7';
    let sp = ' ';
    let uni = '\u{3bb}';
    let emoji = '\u{1F600}';
    println!("{}{}{}", a, z, digit);
    println!("[{}]", sp);
    println!("{} {}", uni, emoji);
    println!("{} {} {}", a as u32, z as u32, digit as u32);
    println!("{} {}", uni as u32, emoji as u32);
    println!("{}", (digit as u8 - b'0') as u32);
    println!("{}", b'A');
    println!("{}", b'\n');
    println!("{}", '\n' as u32);
    println!("{}", '\t' as u32);
    println!("{}", '\\');
    println!("{}", '\'');
    println!("{}", '"');
    println!("{}", '\x41');
    let mut c = 'a' as u8;
    while c <= b'z' {
        if (c - b'a') % 5 == 0 {
            println!("{} = {}", c as char, c);
        }
        c += 1;
    }
    println!("{}", (('a' as u8) + 25) as char);
    println!("{}", ((b'm' - b'a' + 13) % 26 + b'a') as char);
    println!("{}", a < z);
    println!("{}", a == 'a');
    let upper = ((a as u8) - 32) as char;
    println!("{}", upper);
    println!("{}", 'x'.is_alphabetic());
    println!("{}", '9'.is_ascii_digit());
    println!("{}", 'q'.to_ascii_uppercase());
    println!("{}", 'Q'.to_ascii_lowercase());
    println!("{}", emoji.len_utf8());
    println!("{}", uni.len_utf8());
    println!("{}", char::MAX as u32);
}
