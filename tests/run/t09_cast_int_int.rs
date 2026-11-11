// integer `as` casts: truncation, sign extension, zero extension, reinterpretation
fn main() {
    let a: [i64; 8] = [0, 1, -1, 127, 128, 255, 256, -129];
    for &x in a.iter() {
        println!("i64 {}: u8 {} i8 {} u16 {} i16 {} u32 {} i32 {} u64 {}", x, x as u8, x as i8, x as u16, x as i16, x as u32, x as i32, x as u64);
    }
    let b: [u64; 5] = [0x1_0000_0000, 0xFFFF_FFFF, 0x8000_0000_0000_0000, u64::MAX, 0x1234_5678_9abc_def0];
    for &x in b.iter() {
        println!("u64 {}: u8 {} i8 {} u16 {} i16 {} u32 {} i32 {} i64 {}", x, x as u8, x as i8, x as u16, x as i16, x as u32, x as i32, x as i64);
    }
    let c: [i8; 4] = [-128, -1, 0, 127];
    for &x in c.iter() {
        println!("i8 {}: u8 {} u16 {} i16 {} u32 {} i32 {} u64 {} i64 {} u128 {} i128 {}", x, x as u8, x as u16, x as i16, x as u32, x as i32, x as u64, x as i64, x as u128, x as i128);
    }
    let d: [u8; 3] = [0, 128, 255];
    for &x in d.iter() {
        println!("u8 {}: i8 {} i16 {} i32 {} i64 {} char {}", x, x as i8, x as i16, x as i32, x as i64, (x as char) as u32);
    }
    let e: [i128; 4] = [i128::MIN, -1, 1 << 70, i128::MAX];
    for &x in e.iter() {
        println!("i128 {}: u8 {} i32 {} u64 {} i64 {} u128 {}", x, x as u8, x as i32, x as u64, x as i64, x as u128);
    }
    println!("u128::MAX as i128 {} as i8 {}", u128::MAX as i128, u128::MAX as i8);
    println!("usize/isize: {} {} {}", (-1i32) as usize == usize::MAX, usize::MAX as isize, (u32::MAX as usize) as u32);
    // chained casts
    let v: i32 = -1000;
    println!("chain {} {} {}", v as u8 as i32, v as i8 as u32, v as u16 as i16 as i64);
    // casts in arithmetic
    let bytes: [u8; 4] = [0xff, 0x80, 0x7f, 0x01];
    let mut acc: i32 = 0;
    for &b in bytes.iter() {
        acc = acc * 3 + (b as i8) as i32;
        println!("acc {}", acc);
    }
    let w: u16 = 0xfffe;
    println!("u16 {} as i16 {} sign-extended to i64 {}", w, w as i16, w as i16 as i64);
}
