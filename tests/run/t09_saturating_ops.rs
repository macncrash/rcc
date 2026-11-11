// saturating_* arithmetic
fn main() {
    let a: [u8; 5] = [0, 1, 100, 200, 255];
    for &x in a.iter() {
        for &y in a.iter() {
            println!("u8 {} {}: add {} sub {} mul {}", x, y, x.saturating_add(y), x.saturating_sub(y), x.saturating_mul(y));
        }
    }
    let b: [i16; 6] = [i16::MIN, -300, -1, 0, 300, i16::MAX];
    for &x in b.iter() {
        for &y in b.iter() {
            println!("i16 {} {}: add {} sub {} mul {}", x, y, x.saturating_add(y), x.saturating_sub(y), x.saturating_mul(y));
        }
        println!("i16 {}: neg {} abs {} pow3 {}", x, x.saturating_neg(), x.saturating_abs(), x.saturating_pow(3));
    }
    let c: [i64; 3] = [i64::MIN, 0, i64::MAX];
    for &x in c.iter() {
        println!("i64 {}: +1 {} -1 {} *2 {} div-1 {}", x, x.saturating_add(1), x.saturating_sub(1), x.saturating_mul(2), x.saturating_div(-1));
    }
    let mut v: u32 = 1;
    for _ in 0..40 {
        v = v.saturating_mul(3);
    }
    println!("3^40 saturating u32 {}", v);
    println!("u128 {} {}", u128::MAX.saturating_add(5), 5u128.saturating_sub(6));
    println!("i128 {} {}", i128::MAX.saturating_add(1), i128::MIN.saturating_sub(1));
    // mixed signed/unsigned
    println!("add_signed {} {}", 5u32.saturating_add_signed(-10), 5u32.saturating_add_signed(10));
    println!("add_unsigned {}", (-5i32).saturating_add_unsigned(u32::MAX));
}
