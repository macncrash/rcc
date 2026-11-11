// leading_zeros / trailing_zeros / count_ones / count_zeros / leading_ones / trailing_ones
fn main() {
    let a: [u8; 6] = [0, 1, 2, 0x80, 0xF0, 0xFF];
    for &x in a.iter() {
        println!("u8 {}: lz {} tz {} ones {} zeros {} lo {} to {}", x, x.leading_zeros(), x.trailing_zeros(), x.count_ones(), x.count_zeros(), x.leading_ones(), x.trailing_ones());
    }
    let b: [u32; 6] = [0, 1, 0x10, 0x8000_0000, 0xFFFF_0000, u32::MAX];
    for &x in b.iter() {
        println!("u32 {}: lz {} tz {} ones {} zeros {}", x, x.leading_zeros(), x.trailing_zeros(), x.count_ones(), x.count_zeros());
    }
    let c: [i64; 5] = [0, -1, i64::MIN, 12345, -12345];
    for &x in c.iter() {
        println!("i64 {}: lz {} tz {} ones {} zeros {} lo {}", x, x.leading_zeros(), x.trailing_zeros(), x.count_ones(), x.count_zeros(), x.leading_ones());
    }
    let d: [u128; 4] = [0, 1, 1u128 << 100, u128::MAX];
    for &x in d.iter() {
        println!("u128 {}: lz {} tz {} ones {}", x, x.leading_zeros(), x.trailing_zeros(), x.count_ones());
    }
    let e: [u16; 3] = [0, 0x0100, 0xFFFE];
    for &x in e.iter() {
        println!("u16 {}: lz {} tz {} ones {}", x, x.leading_zeros(), x.trailing_zeros(), x.count_ones());
    }
    // log2 via leading_zeros, power-of-two checks
    for x in [1u64, 2, 3, 4, 1000, 1 << 40, u64::MAX] {
        println!("{} ilog2 {} lzlog {} is_pow2 {} next_pow2 {}", x, x.ilog2(), 63 - x.leading_zeros(), x.is_power_of_two(), if x <= 1 << 62 { x.next_power_of_two() } else { 0 });
    }
    println!("ilog10 {} {} {}", 999u32.ilog10(), 1000u32.ilog10(), u64::MAX.ilog10());
    println!("BITS {} {} {} {} {}", u8::BITS, i16::BITS, u32::BITS, i64::BITS, u128::BITS);
}
