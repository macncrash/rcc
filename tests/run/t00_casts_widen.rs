fn main() {
    let a: u8 = 200;
    let b: i8 = -100;
    println!("{}", a as u16);
    println!("{}", a as i16);
    println!("{}", a as u32);
    println!("{}", a as i64);
    println!("{}", a as u128);
    println!("{}", b as i16);
    println!("{}", b as i32);
    println!("{}", b as i64);
    println!("{}", b as i128);
    // sign extension then reinterpret as unsigned
    println!("{}", b as u16);
    println!("{}", b as u32);
    println!("{}", b as u64);
    println!("{}", b as usize);
    let c: i32 = -1;
    println!("{}", c as u64);
    println!("{}", c as i64);
    println!("{}", c as u128);
    let d: u32 = 3_000_000_000;
    println!("{}", d as i64);
    println!("{}", d as u64);
    println!("{}", d as i32);
    let e: i16 = -2;
    println!("{}", e as u32);
    println!("{}", (e as u16) as u32);
    println!("{}", e as isize);
    println!("{}", true as i32);
    println!("{}", false as u8);
    println!("{}", 'A' as u32);
    println!("{}", 'z' as u8);
    println!("{}", 65u8 as char);
    println!("{}", 5i32 as f64);
    println!("{}", -7i64 as f64);
    println!("{}", u64::MAX as f64);
}
