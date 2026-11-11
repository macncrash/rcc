fn main() {
    let a: u8 = 250;
    println!("{}", a.wrapping_add(10));
    println!("{}", a.wrapping_mul(3));
    println!("{}", 5u8.wrapping_sub(10));
    let b: i8 = 127;
    println!("{}", b.wrapping_add(1));
    println!("{}", i8::MIN.wrapping_sub(1));
    println!("{}", i8::MIN.wrapping_neg());
    println!("{}", i8::MIN.wrapping_abs());
    println!("{}", (-128i8).wrapping_div(-1));
    println!("{}", (-128i8).wrapping_rem(-1));
    let c: i32 = i32::MAX;
    println!("{}", c.wrapping_add(c));
    println!("{}", c.wrapping_mul(3));
    println!("{}", i32::MIN.wrapping_sub(1));
    let d: u64 = u64::MAX;
    println!("{}", d.wrapping_add(2));
    println!("{}", d.wrapping_mul(d));
    println!("{}", 0u64.wrapping_sub(1));
    println!("{}", 1u32.wrapping_shl(35));
    println!("{}", 0x8000_0000u32.wrapping_shr(33));
    // hash-like mixing with wrapping ops
    let mut h: u64 = 0xcbf29ce484222325;
    let mut i: u64 = 0;
    while i < 10 {
        h ^= i;
        h = h.wrapping_mul(0x100000001b3);
        println!("h{} = {}", i, h);
        i += 1;
    }
    let mut x: u32 = 1;
    let mut k = 0;
    while k < 8 {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        println!("lcg {}", x);
        k += 1;
    }
    // plain operator wrapping: relies on overflow-checks=off (release semantics)
    let mut y: u8 = 200;
    y = y + 100;
    println!("plain wrap {}", y);
    let mut z: i32 = i32::MAX;
    z = z + 1;
    println!("plain wrap {}", z);
}
