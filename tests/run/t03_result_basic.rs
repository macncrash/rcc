enum ParseErr {
    Empty,
    BadChar(u8, usize),
    Overflow,
}

fn parse_u32(s: &str) -> Result<u32, ParseErr> {
    let b = s.as_bytes();
    if b.len() == 0 {
        return Err(ParseErr::Empty);
    }
    let mut v: u64 = 0;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c < b'0' || c > b'9' {
            return Err(ParseErr::BadChar(c, i));
        }
        v = v * 10 + (c - b'0') as u64;
        if v > u32::MAX as u64 {
            return Err(ParseErr::Overflow);
        }
        i += 1;
    }
    Ok(v as u32)
}

fn report(s: &str) {
    match parse_u32(s) {
        Ok(v) => println!("'{}' -> {}", s, v),
        Err(ParseErr::Empty) => println!("'{}' -> empty", s),
        Err(ParseErr::BadChar(c, i)) => println!("'{}' -> bad char {} at {}", s, c, i),
        Err(ParseErr::Overflow) => println!("'{}' -> overflow", s),
    }
}

fn main() {
    report("123");
    report("");
    report("12a4");
    report("4294967295");
    report("4294967296");
    report("0007");
    let r: Result<i32, &str> = Ok(3);
    let e: Result<i32, &str> = Err("bad");
    println!("{} {} {} {}", r.is_ok(), r.is_err(), e.is_ok(), e.is_err());
    println!("{} {}", r.unwrap(), e.unwrap_or(-1));
    println!("{}", e.unwrap_err());
    match r.ok() {
        Some(v) => println!("ok() {}", v),
        None => println!("ok() none"),
    }
    match e.err() {
        Some(m) => println!("err() {}", m),
        None => println!("err() none"),
    }
    let rr: Result<Result<u8, u8>, u8> = Ok(Err(4));
    match rr {
        Ok(Ok(v)) => println!("okok {}", v),
        Ok(Err(v)) => println!("okerr {}", v),
        Err(v) => println!("err {}", v),
    }
}
