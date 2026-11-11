fn main() {
    let a: u32 = 1;
    let mut i = 0;
    while i < 32 {
        if i % 5 == 0 {
            println!("1 << {} = {}", i, a << i);
        }
        i += 1;
    }
    let neg: i32 = -256;
    println!("{}", neg >> 1);
    println!("{}", neg >> 4);
    println!("{}", neg >> 31);
    println!("{}", (neg as u32) >> 4);
    println!("{}", (neg as u32) >> 31);
    println!("{}", neg << 3);
    let b: i8 = -128;
    println!("{}", b >> 7);
    println!("{}", (b as u8) >> 7);
    let c: u8 = 0x81;
    println!("{}", c << 1);
    println!("{}", c >> 1);
    let d: i64 = -1;
    println!("{}", d << 63);
    println!("{}", (d as u64) >> 1);
    let e: u64 = 0x0123_4567_89AB_CDEF;
    let mut s = 0;
    while s < 64 {
        println!("e >> {} = {}", s, e >> s);
        s += 12;
    }
    let shift: u8 = 3;
    println!("{}", 100u16 << shift);
    let shift64: u64 = 2;
    println!("{}", 100i32 >> shift64);
    let k: i16 = 0x4000;
    println!("{}", k << 1);
    println!("{}", 1usize << 40);
}
