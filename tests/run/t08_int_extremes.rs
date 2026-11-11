fn main() {
    println!("{} {}", i8::MIN, i8::MAX);
    println!("{} {}", i16::MIN, i16::MAX);
    println!("{} {}", i32::MIN, i32::MAX);
    println!("{} {}", i64::MIN, i64::MAX);
    println!("{} {}", i128::MIN, i128::MAX);
    println!("{} {} {} {} {}", u8::MAX, u16::MAX, u32::MAX, u64::MAX, u128::MAX);
    println!("{} {}", isize::MIN, usize::MAX);
    println!("{:?} {:?}", i128::MIN, u128::MAX);
    println!("[{:>45}]", i128::MIN);
    println!("[{:<41}]", u128::MAX);
    println!("[{:+}]", i128::MAX);
    let parsed: i64 = "-9223372036854775808".parse().unwrap();
    println!("parsed {} eq={}", parsed, parsed == i64::MIN);
    let p128: u128 = "340282366920938463463374607431768211455".parse().unwrap();
    println!("parsed {} eq={}", p128, p128 == u128::MAX);
    let n128: i128 = "-170141183460469231731687303715884105728".parse().unwrap();
    println!("parsed {}", n128);
    println!("{:?}", "9223372036854775808".parse::<i64>().is_err());
    println!("{:?}", "256".parse::<u8>().map_err(|e| e.to_string()));
    println!("{:?}", "-1".parse::<u32>().map_err(|e| e.to_string()));
    println!("{:?}", "".parse::<i32>().map_err(|e| e.to_string()));
    println!("{:?}", "+77".parse::<i32>());
    println!("{:?}", "-0".parse::<i8>());
    println!("{:?}", " 1".parse::<i32>().is_err());
    let mut x: u64 = 1;
    for _ in 0..20 {
        print!("{} ", x);
        x = x * 10 + 1;
    }
    println!();
    let mut y: i128 = -1;
    for _ in 0..5 {
        y *= 1_000_000_007;
        println!("{}", y);
    }
    println!("{}", u128::MAX / 3);
    println!("{}", (u64::MAX as u128) * (u64::MAX as u128));
    println!("{}", i64::MIN.to_string().len());
    println!("{}", 0i128);
}
