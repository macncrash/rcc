fn main() {
    let a: u32 = 0x12345678;
    println!("{}", a as u8);
    println!("{}", a as u16);
    println!("{}", a as i8);
    println!("{}", a as i16);
    let b: i32 = -129;
    println!("{}", b as i8);
    println!("{}", b as u8);
    let c: i64 = 0x1_0000_0001;
    println!("{}", c as i32);
    println!("{}", c as u32);
    let d: u64 = u64::MAX;
    println!("{}", d as i64);
    println!("{}", d as u32);
    println!("{}", d as i8);
    let e: i64 = -1;
    println!("{}", e as u8);
    println!("{}", e as u16);
    println!("{}", e as u32);
    println!("{}", e as u64);
    let f: u16 = 40000;
    println!("{}", f as i16);
    println!("{}", f as u8);
    let g: i128 = -170141183460469231731687303715884105728;
    println!("{}", g as i64);
    println!("{}", g as u64);
    println!("{}", (g + 12345) as u16);
    let h: f64 = 3.99;
    println!("{}", h as i32);
    println!("{}", -h as i32);
    println!("{}", h as u8);
    let mut i: u32 = 250;
    while i < 262 {
        println!("{} as u8 = {}, as i8 = {}", i, i as u8, i as i8);
        i += 3;
    }
    println!("{}", 0x41u32 as u8 as char);
    println!("{}", 300i32 as u8 as i32);
}
