// wrapping_* methods (explicit, independent of overflow-checks)
fn main() {
    let a: [u8; 4] = [0, 1, 128, 255];
    for &x in a.iter() {
        for &y in a.iter() {
            println!(
                "u8 {} {}: add {} sub {} mul {} neg {}",
                x,
                y,
                x.wrapping_add(y),
                x.wrapping_sub(y),
                x.wrapping_mul(y),
                x.wrapping_neg()
            );
        }
    }
    let b: [i8; 5] = [i8::MIN, -1, 0, 1, i8::MAX];
    for &x in b.iter() {
        for &y in b.iter() {
            let d = if y != 0 { x.wrapping_div(y) } else { 0 };
            let r = if y != 0 { x.wrapping_rem(y) } else { 0 };
            println!(
                "i8 {} {}: add {} sub {} mul {} div {} rem {}",
                x,
                y,
                x.wrapping_add(y),
                x.wrapping_sub(y),
                x.wrapping_mul(y),
                d,
                r
            );
        }
        println!("i8 {}: neg {} abs {}", x, x.wrapping_neg(), x.wrapping_abs());
    }
    let mut h: u32 = 2166136261;
    for c in 0u32..50 {
        h ^= c;
        h = h.wrapping_mul(16777619);
    }
    println!("fnv32 {}", h);
    let mut h64: u64 = 0xcbf29ce484222325;
    for c in 0u64..50 {
        h64 ^= c;
        h64 = h64.wrapping_mul(0x100000001b3);
    }
    println!("fnv64 {}", h64);
    let s: u32 = 1;
    for sh in [0u32, 1, 31, 32, 33, 63, 64] {
        println!("shl {} -> {} shr -> {}", sh, s.wrapping_shl(sh), 0x80000000u32.wrapping_shr(sh));
    }
    let i: i64 = i64::MIN;
    println!("i64 min: neg {} abs {} sub1 {} pow2 {}", i.wrapping_neg(), i.wrapping_abs(), i.wrapping_sub(1), i.wrapping_pow(2));
    println!("3^41 wrapping u64 {}", 3u64.wrapping_pow(41));
    println!("7^100 wrapping u32 {}", 7u32.wrapping_pow(100));
    let mut acc: u16 = 1;
    for k in 1u16..30 {
        acc = acc.wrapping_mul(k).wrapping_add(k);
        print!("{} ", acc);
    }
    println!();
}
