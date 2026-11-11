// MIN/MAX for every integer type, and arithmetic around the limits
fn main() {
    println!("u8 {} {}", u8::MIN, u8::MAX);
    println!("i8 {} {}", i8::MIN, i8::MAX);
    println!("u16 {} {}", u16::MIN, u16::MAX);
    println!("i16 {} {}", i16::MIN, i16::MAX);
    println!("u32 {} {}", u32::MIN, u32::MAX);
    println!("i32 {} {}", i32::MIN, i32::MAX);
    println!("u64 {} {}", u64::MIN, u64::MAX);
    println!("i64 {} {}", i64::MIN, i64::MAX);
    println!("u128 {} {}", u128::MIN, u128::MAX);
    println!("i128 {} {}", i128::MIN, i128::MAX);
    println!("usize max == u64 max: {}", usize::MAX as u64 == u64::MAX);
    println!("isize min == i64 min: {}", isize::MIN as i64 == i64::MIN);
    println!("hex {:x} {:x} {:x}", i8::MIN, i32::MIN, i64::MIN);
    println!("{:x} {:x}", i128::MIN, u128::MAX);
    // overflow-checks=off: plain + wraps (we rely on release semantics here deliberately,
    // but only through values the compiler cannot see as constants)
    let vals: [i32; 2] = [i32::MAX, i32::MIN];
    for &v in vals.iter() {
        println!("{}: wrapping+1 {} wrapping-1 {}", v, v.wrapping_add(1), v.wrapping_sub(1));
    }
    let parse_tests = ["127", "128", "-128", "-129", "255", "0", "-0", "+5", "", "12a"];
    for s in parse_tests.iter() {
        let a = s.parse::<i8>();
        let b = s.parse::<u8>();
        println!(
            "{:?}: i8 {} u8 {}",
            s,
            match a { Ok(v) => format!("{}", v), Err(_) => String::from("err") },
            match b { Ok(v) => format!("{}", v), Err(_) => String::from("err") }
        );
    }
    println!("i64 min parse {}", "-9223372036854775808".parse::<i64>().unwrap());
    println!("u64 max parse {}", "18446744073709551615".parse::<u64>().unwrap());
    println!("u64 overflow parse is_err {}", "18446744073709551616".parse::<u64>().is_err());
    println!("from_str_radix {} {} {}", i64::from_str_radix("-ff", 16).unwrap(), u32::from_str_radix("777", 8).unwrap(), u8::from_str_radix("11111111", 2).unwrap());
    println!("i32 try_from {:?} {:?}", i32::try_from(3_000_000_000u64).is_err(), u8::try_from(200i32));
    println!("u16 try_from {:?} {:?}", u16::try_from(-1i32).is_err(), i8::try_from(-128i64));
    println!("from {} {} {}", u64::from(7u8), i64::from(-3i32), u128::from(u64::MAX));
}
