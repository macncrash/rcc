// shifts: arithmetic vs logical, by runtime amounts, checked/wrapping for oversized amounts
fn main() {
    let neg: i32 = -12345;
    let pos: u32 = 0xF000_000F;
    for s in 0u32..32 {
        println!("s {:2}: i32>> {} u32>> {} i32<< {} u32<< {}", s, neg >> s, pos >> s, neg << s, pos << s);
    }
    let a: i8 = -128;
    for s in 0u32..8 {
        println!("i8 {}>>{} = {} u8 {}>>{} = {}", a, s, a >> s, a as u8, s, (a as u8) >> s);
    }
    let amounts: [u32; 5] = [0, 31, 32, 33, 64];
    for &s in amounts.iter() {
        println!("checked_shl({}) {:?} checked_shr({}) {:?} wrapping_shl {} wrapping_shr {}", s, 1u32.checked_shl(s), s, 0x8000_0000u32.checked_shr(s), 1u32.wrapping_shl(s), 0x8000_0000u32.wrapping_shr(s));
    }
    // shift amounts of differing types
    let s8: u8 = 3;
    let s64: u64 = 5;
    let si: i32 = 2;
    println!("mixed {} {} {} {}", 1u64 << s8, 1u8 << si, 0x100i16 >> s64, (-1i64) << s8);
    // bit field packing
    let mut packed: u64 = 0;
    let fields: [(u64, u32, u32); 4] = [(5, 0, 3), (100, 3, 7), (1, 10, 1), (0xABCD, 11, 16)];
    for &(v, off, width) in fields.iter() {
        let mask = (1u64 << width) - 1;
        packed |= (v & mask) << off;
        println!("pack {} at {} width {} -> {:x}", v, off, width, packed);
    }
    for &(_, off, width) in fields.iter() {
        let mask = (1u64 << width) - 1;
        println!("unpack at {} -> {}", off, (packed >> off) & mask);
    }
    // sign extension of an n-bit field
    for &bits in [4u32, 8, 12, 20].iter() {
        let raw: u32 = 0xFFF8_FFF8 & ((1u32 << bits) - 1);
        let sh = 32 - bits;
        let sext = ((raw << sh) as i32) >> sh;
        println!("sext {} bits of {:x} = {}", bits, raw, sext);
    }
    let x: u128 = 1;
    println!("u128 shifts {} {} {}", x << 127, (x << 127) >> 127, (u128::MAX >> 1) as i128);
    let y: i128 = -1;
    println!("i128 >> 127 {} >> 1 {}", y >> 127, i128::MIN >> 1);
    let mut z: u32 = 1;
    let mut i = 0;
    while z != 0 {
        z <<= 3;
        i += 1;
    }
    println!("u32 1<<=3 reaches 0 after {} steps", i);
}
