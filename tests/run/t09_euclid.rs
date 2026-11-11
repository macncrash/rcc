// / and % truncate toward zero; div_euclid / rem_euclid; checked_rem_euclid
fn main() {
    let a: [i32; 7] = [-7, -6, -1, 0, 1, 6, 7];
    let b: [i32; 4] = [-3, -2, 2, 3];
    for &x in a.iter() {
        for &y in b.iter() {
            println!("{} {}: / {} % {} div_euclid {} rem_euclid {}", x, y, x / y, x % y, x.div_euclid(y), x.rem_euclid(y));
        }
    }
    let c: [i64; 3] = [i64::MIN, -1, i64::MAX];
    for &x in c.iter() {
        println!("i64 {}: rem_euclid 10 {} div_euclid 10 {} wrapping_rem_euclid -1 {}", x, x.rem_euclid(10), x.div_euclid(10), x.wrapping_rem_euclid(-1));
    }
    match i32::MIN.checked_div_euclid(-1) {
        Some(v) => println!("checked_div_euclid {}", v),
        None => println!("checked_div_euclid None"),
    }
    match 5i32.checked_rem_euclid(0) {
        Some(v) => println!("checked_rem_euclid {}", v),
        None => println!("checked_rem_euclid None"),
    }
    println!("u32 div_euclid {} rem_euclid {}", 17u32.div_euclid(5), 17u32.rem_euclid(5));
    let fs: [f64; 4] = [-7.5, -1.0, 1.0, 7.5];
    for &x in fs.iter() {
        println!("f64 {}: % 2 = {} rem_euclid 2 = {} div_euclid 2 = {}", x, x % 2.0, x.rem_euclid(2.0), x.div_euclid(2.0));
    }
    // circular index arithmetic
    let n: i32 = 5;
    for i in -12i32..13 {
        print!("{} ", i.rem_euclid(n));
    }
    println!();
    // i8 / i16 remainder sign behaviour
    for x in [-128i8, -100, -1, 1, 127] {
        println!("i8 {} % 7 = {} / 7 = {} rem_euclid {}", x, x % 7, x / 7, x.rem_euclid(7));
    }
    for x in [-32768i16, -3, 3, 32767] {
        println!("i16 {} % -10 = {} / -10 = {}", x, x % -10, x / -10);
    }
}
