// char <-> u32/u8, bool -> int, char::from_u32, from_digit, to_digit
fn main() {
    let cs: [char; 7] = ['\0', 'A', 'z', '~', '\u{7f}', '\u{e9}', '\u{1F600}'];
    for &c in cs.iter() {
        println!("char {:?} as u32 {} as u8 {} as u16 {} len_utf8 {}", c, c as u32, c as u8, c as u16, c.len_utf8());
    }
    for b in [0u8, 48, 65, 97, 200, 255] {
        let c = b as char;
        println!("u8 {} as char {:?} ({})", b, c, c as u32);
    }
    for u in [65u32, 0xD7FF, 0xD800, 0xDFFF, 0xE000, 0x10FFFF, 0x110000] {
        match char::from_u32(u) {
            Some(c) => println!("from_u32 {:x} = {:?}", u, c),
            None => println!("from_u32 {:x} = None", u),
        }
    }
    for d in 0u32..17 {
        match char::from_digit(d, 16) {
            Some(c) => print!("{}", c),
            None => print!("[none]"),
        }
    }
    println!();
    for c in ['0', '7', '9', 'a', 'F', 'z', 'G'] {
        println!("{} to_digit10 {:?} to_digit16 {:?} to_digit36 {:?}", c, c.to_digit(10), c.to_digit(16), c.to_digit(36));
    }
    let t = true;
    let f = false;
    println!("bool: {} {} {} {} {}", t as u8, f as u8, t as i32, t as u64, t as i128);
    println!("bool arith {}", (t as i32) * 10 + (f as i32) * 5 + (3 > 2) as i32);
    let mut count = 0u32;
    for i in 0..20 {
        count += (i % 3 == 0) as u32;
    }
    println!("multiples of 3 below 20: {}", count);
    // char arithmetic via u32
    let mut s = String::new();
    for i in 0..26u32 {
        let c = char::from_u32('a' as u32 + (i * 7) % 26).unwrap();
        s.push(c);
    }
    println!("{}", s);
    println!("u8 'a' + 2 = {}", (b'a' + 2) as char);
    println!("upper {} lower {}", ('q' as u8 - 32) as char, ('Q' as u8 | 0x20) as char);
    println!("char cmp {} {}", 'a' < 'b', '\u{e9}' > 'z');
    println!("char::MAX {}", char::MAX as u32);
}
