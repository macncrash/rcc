fn big(x: u128) -> &'static str {
    match x {
        0 => "zero",
        1..=0xffff_ffff_ffff_ffff => "fits u64",
        0x1_0000_0000_0000_0000..=0xffff_ffff_ffff_ffff_ffff_ffff_ffff_fffe => "big",
        u128::MAX => "max",
    }
}

fn sbig(x: i128) -> i32 {
    match x {
        i128::MIN => -2,
        i128::MIN..=-1 => -1,
        0 => 0,
        _ => 1,
    }
}

fn byte(b: u8) -> &'static str {
    match b {
        b'a' | b'e' | b'i' | b'o' | b'u' => "vowel",
        b'a'..=b'z' => "consonant",
        0x80..=0xff => "high",
        _ => "other",
    }
}

fn float_cmp(f: f64) -> i32 {
    if f < 0.0 { -1 } else if f == 0.0 { 0 } else { 1 }
}

fn main() {
    let xs = [0u128, 5, 1 << 64, u128::MAX - 1, u128::MAX];
    for i in 0..xs.len() {
        println!("{} {}", xs[i], big(xs[i]));
    }
    let ys = [i128::MIN, i128::MIN + 1, -1, 0, i128::MAX];
    for i in 0..ys.len() {
        println!("{} {}", ys[i], sbig(ys[i]));
    }
    let bs = b"apple\xff";
    for i in 0..bs.len() {
        println!("{} {}", bs[i], byte(bs[i]));
    }
    println!("{} {} {}", float_cmp(-0.5), float_cmp(0.0), float_cmp(3.5));
    let u: usize = 3;
    let r = match u {
        0 => 'a',
        1..=2 => 'b',
        usize::MAX => 'z',
        _ => 'c',
    };
    println!("{}", r);
}
