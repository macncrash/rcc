fn inc(x: i32) -> i32 {
    x + 1
}

fn safe_sqrt(x: i32) -> Result<i32, &'static str> {
    if x < 0 {
        return Err("negative");
    }
    let mut r = 0;
    while (r + 1) * (r + 1) <= x {
        r += 1;
    }
    if r * r == x { Ok(r) } else { Err("not square") }
}

fn code(e: &'static str) -> i32 {
    e.len() as i32
}

fn recover(_e: &'static str) -> Result<i32, &'static str> {
    Ok(0)
}

fn show(r: Result<i32, &'static str>) {
    match r {
        Ok(v) => println!("Ok {}", v),
        Err(e) => println!("Err {}", e),
    }
}

fn main() {
    let inputs = [16, 15, -4, 0, 81];
    for i in 0..inputs.len() {
        let x = inputs[i];
        let r = safe_sqrt(x);
        show(r);
        show(safe_sqrt(x).map(inc));
        show(safe_sqrt(x).and_then(safe_sqrt));
        show(safe_sqrt(x).or_else(recover));
        println!("unwrap_or {}", safe_sqrt(x).unwrap_or(-1));
        println!("map_err {}", match safe_sqrt(x).map_err(code) {
            Ok(v) => v * 100,
            Err(c) => -c,
        });
        println!("map_or {}", safe_sqrt(x).map_or(7, inc));
        println!("is_ok_and {}", safe_sqrt(x).is_ok_and(is_pos));
    }
    let r: Result<i32, &str> = Ok(5);
    let as_opt: Option<i32> = r.ok();
    println!("{}", as_opt.unwrap());
    let o: Option<Result<i32, &str>> = Some(Ok(3));
    match o.transpose() {
        Ok(Some(v)) => println!("transposed {}", v),
        Ok(None) => println!("none"),
        Err(e) => println!("err {}", e),
    }
    println!("{}", safe_sqrt(49).expect("should be square"));
    println!("{}", safe_sqrt(50).unwrap_or_default());
}

fn is_pos(x: i32) -> bool {
    x > 0
}
