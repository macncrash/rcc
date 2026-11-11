// checked_* arithmetic on several integer widths
fn show_u8(name: &str, r: Option<u8>) {
    match r {
        Some(v) => println!("{} = Some({})", name, v),
        None => println!("{} = None", name),
    }
}
fn show_i32(name: &str, r: Option<i32>) {
    match r {
        Some(v) => println!("{} = Some({})", name, v),
        None => println!("{} = None", name),
    }
}
fn show_u64(name: &str, r: Option<u64>) {
    match r {
        Some(v) => println!("{} = Some({})", name, v),
        None => println!("{} = None", name),
    }
}

fn main() {
    let xs: [u8; 5] = [0, 1, 100, 200, 255];
    for &a in xs.iter() {
        for &b in xs.iter() {
            println!("u8 {} {}", a, b);
            show_u8("  add", a.checked_add(b));
            show_u8("  sub", a.checked_sub(b));
            show_u8("  mul", a.checked_mul(b));
            show_u8("  div", a.checked_div(b));
            show_u8("  rem", a.checked_rem(b));
        }
    }
    let ys: [i32; 6] = [i32::MIN, -7, -1, 0, 3, i32::MAX];
    for &a in ys.iter() {
        for &b in ys.iter() {
            println!("i32 {} {}", a, b);
            show_i32("  add", a.checked_add(b));
            show_i32("  sub", a.checked_sub(b));
            show_i32("  mul", a.checked_mul(b));
            show_i32("  div", a.checked_div(b));
            show_i32("  rem", a.checked_rem(b));
        }
        show_i32("  neg", a.checked_neg());
        show_i32("  abs", a.checked_abs());
        show_i32("  shl31", a.checked_shl(31));
        show_i32("  shl32", a.checked_shl(32));
        show_i32("  shr5", a.checked_shr(5));
        show_i32("  pow3", a.checked_pow(3));
    }
    let big: [u64; 4] = [0, 1, u64::MAX / 2, u64::MAX];
    for &a in big.iter() {
        show_u64("u64 add1", a.checked_add(1));
        show_u64("u64 mul2", a.checked_mul(2));
        show_u64("u64 sub1", a.checked_sub(1));
        show_u64("u64 pow2", a.checked_pow(2));
        show_u64("u64 next_pow2", a.checked_next_power_of_two());
    }
}
