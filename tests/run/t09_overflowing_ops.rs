// overflowing_* returns (value, overflowed)
fn main() {
    let a: [u32; 4] = [0, 1, 0x8000_0000, u32::MAX];
    for &x in a.iter() {
        for &y in a.iter() {
            let (s, so) = x.overflowing_add(y);
            let (d, dof) = x.overflowing_sub(y);
            let (m, mo) = x.overflowing_mul(y);
            println!("u32 {} {}: add ({}, {}) sub ({}, {}) mul ({}, {})", x, y, s, so, d, dof, m, mo);
        }
        let (n, no) = x.overflowing_neg();
        println!("u32 {} neg ({}, {})", x, n, no);
    }
    let b: [i32; 4] = [i32::MIN, -1, 1, i32::MAX];
    for &x in b.iter() {
        for &y in b.iter() {
            let (q, qo) = x.overflowing_div(y);
            let (r, ro) = x.overflowing_rem(y);
            let (s, so) = x.overflowing_add(y);
            println!("i32 {} {}: div ({}, {}) rem ({}, {}) add ({}, {})", x, y, q, qo, r, ro, s, so);
        }
        let (n, no) = x.overflowing_neg();
        let (ab, abo) = x.overflowing_abs();
        let (p, po) = x.overflowing_pow(2);
        println!("i32 {}: neg ({}, {}) abs ({}, {}) pow2 ({}, {})", x, n, no, ab, abo, p, po);
    }
    for sh in [0u32, 7, 8, 9, 100] {
        let (v, o) = 1u8.overflowing_shl(sh);
        let (w, wo) = 0x80u8.overflowing_shr(sh);
        println!("u8 shl {} -> ({}, {}) shr -> ({}, {})", sh, v, o, w, wo);
    }
    // multi-word add with carries
    let x: [u64; 3] = [u64::MAX, u64::MAX, 1];
    let y: [u64; 3] = [1, 0, 2];
    let mut out = [0u64; 4];
    let mut carry = false;
    for i in 0..3 {
        let (s1, c1) = x[i].overflowing_add(y[i]);
        let (s2, c2) = s1.overflowing_add(carry as u64);
        out[i] = s2;
        carry = c1 || c2;
    }
    out[3] = carry as u64;
    println!("bignum {} {} {} {}", out[0], out[1], out[2], out[3]);
}
