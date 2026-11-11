fn main() {
    let a: i32 = "42".parse().unwrap();
    let b = "-17".parse::<i64>().unwrap();
    let c = "255".parse::<u8>().unwrap();
    let d = "3.5".parse::<f64>().unwrap();
    let e = "true".parse::<bool>().unwrap();
    let f = "x".parse::<char>().unwrap();
    println!("{} {} {} {} {} {}", a, b, c, d, e, f);
    println!("{:?}", "256".parse::<u8>().is_err());
    println!("{:?}", "".parse::<i32>().is_err());
    println!("{:?}", "12a".parse::<i32>().ok());
    println!("{:?}", "+7".parse::<i32>());
    println!("{:?}", " 7".parse::<i32>().is_ok());
    println!("{:?}", "1e3".parse::<f64>());
    println!("{:?}", "-0.25".parse::<f32>());
    println!("{:?}", u64::from_str_radix("ff", 16));
    println!("{:?}", i32::from_str_radix("-101", 2));
    println!("{:?}", u32::from_str_radix("777", 8));
    println!("{:?}", "18446744073709551615".parse::<u64>());
    println!("{:?}", "-9223372036854775808".parse::<i64>());
    let nums = ["1", "two", "3", "-4", "5x"];
    let good: Vec<i32> = nums.iter().filter_map(|s| s.parse().ok()).collect();
    println!("{:?}", good);
    let all: Result<Vec<i32>, _> = nums.iter().map(|s| s.parse::<i32>()).collect();
    println!("all ok {}", all.is_ok());
    let some: Result<Vec<i32>, std::num::ParseIntError> =
        ["1", "2"].iter().map(|s| s.parse::<i32>()).collect();
    println!("{:?}", some);
    match "abc".parse::<u32>() {
        Ok(v) => println!("ok {}", v),
        Err(e) => println!("err {}", e),
    }
    match "".parse::<u32>() {
        Ok(v) => println!("ok {}", v),
        Err(e) => println!("err {}", e),
    }
    let hex = "0x1F";
    let v = if let Some(h) = hex.strip_prefix("0x") {
        i64::from_str_radix(h, 16).unwrap()
    } else {
        hex.parse().unwrap()
    };
    println!("{}", v);
}
