// rotate_left/right, swap_bytes, reverse_bits, to_be/from_be
fn main() {
    let x: u32 = 0x1234_5678;
    for n in [0u32, 1, 4, 8, 16, 31, 32, 36] {
        println!("u32 rotl {} = {:x} rotr = {:x}", n, x.rotate_left(n), x.rotate_right(n));
    }
    let y: u8 = 0b1000_0011;
    for n in 0u32..9 {
        println!("u8 rotl {} = {} rotr = {}", n, y.rotate_left(n), y.rotate_right(n));
    }
    let z: i16 = -2;
    println!("i16 rotl3 {} rotr3 {}", z.rotate_left(3), z.rotate_right(3));
    let w: u64 = 0x0123_4567_89ab_cdef;
    println!("u64 swap {:x} rev {:x}", w.swap_bytes(), w.reverse_bits());
    println!("u64 rotl 12 {:x} rotr 12 {:x}", w.rotate_left(12), w.rotate_right(12));
    println!("u32 swap {:x} rev {:x}", x.swap_bytes(), x.reverse_bits());
    println!("u16 swap {:x} rev {:x}", 0xabcdu16.swap_bytes(), 0x0001u16.reverse_bits());
    println!("i32 swap {} rev {}", (-2i32).swap_bytes(), 1i32.reverse_bits());
    println!("u8 swap {} rev {}", 0x12u8.swap_bytes(), 0x01u8.reverse_bits());
    let v: u128 = 0x0011_2233_4455_6677_8899_aabb_ccdd_eeff;
    println!("u128 swap {:x}", v.swap_bytes());
    println!("u128 rotl 4 {:x} rotr 68 {:x}", v.rotate_left(4), v.rotate_right(68));
    println!("u128 rev {:x}", v.reverse_bits());
    // to_be / from_be are host-endian dependent in intermediate form, but round trip is not
    println!("be round trip {}", u32::from_be(x.to_be()) == x);
    println!("le round trip {}", u64::from_le(w.to_le()) == w);
    // a hash mixing function
    let mut h: u64 = 0;
    for i in 0u64..20 {
        h = (h.rotate_left(5) ^ i).wrapping_mul(0x517cc1b727220a95);
        println!("mix {} {:x}", i, h);
    }
}
