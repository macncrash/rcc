fn main() {
    let a: u32 = 0b1100_1010;
    let b: u32 = 0b1010_0110;
    println!("{}", a & b);
    println!("{}", a | b);
    println!("{}", a ^ b);
    println!("{}", !a);
    println!("{}", !0u8);
    println!("{}", !0i32);
    println!("{}", !5i64);
    let c: i32 = -12345;
    println!("{}", c & 0xFF);
    println!("{}", c | 0x0F);
    println!("{}", c ^ -1);
    let mut x: u64 = 0xDEAD_BEEF_CAFE_BABE;
    println!("{}", x);
    println!("{}", x & 0xFFFF);
    println!("{}", (x >> 16) & 0xFFFF);
    println!("{}", (x >> 32) & 0xFFFF);
    println!("{}", x >> 48);
    let mut pop = 0;
    while x != 0 {
        pop += x & 1;
        x >>= 1;
    }
    println!("popcount = {}", pop);
    let mut v: u32 = 0;
    let mut i = 0;
    while i < 32 {
        if i % 3 == 0 {
            v |= 1 << i;
        }
        i += 1;
    }
    println!("every third bit = {}", v);
    let low = v & v.wrapping_neg();
    println!("lowest set = {}", low);
    let cleared = v & (v - 1);
    println!("cleared lowest = {}", cleared);
    let aligned = (1000u64 + 15) & !15;
    println!("aligned = {}", aligned);
    let m: u8 = 0b1111_0000;
    println!("{}", m & !0b0011_0000);
    println!("{}", (0xABu16 << 8) | 0xCD);
}
