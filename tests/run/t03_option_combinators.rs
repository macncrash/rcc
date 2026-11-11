fn double(x: i32) -> i32 {
    x * 2
}

fn half(x: i32) -> Option<i32> {
    if x % 2 == 0 { Some(x / 2) } else { None }
}

fn is_big(x: &i32) -> bool {
    *x > 10
}

fn fallback() -> i32 {
    -7
}

fn main() {
    let a = Some(12);
    let n: Option<i32> = None;
    println!("{}", a.map(double).unwrap());
    println!("{}", n.map(double).unwrap_or(0));
    println!("{}", a.and_then(half).and_then(half).unwrap_or(-1));
    println!("{}", a.and_then(half).and_then(half).and_then(half).unwrap_or(-1));
    println!("{}", n.unwrap_or_else(fallback));
    println!("{}", a.unwrap_or_default() + n.unwrap_or_default());
    println!("{}", a.filter(is_big).is_some());
    println!("{}", Some(3).filter(is_big).is_some());
    println!("{}", n.or(Some(5)).unwrap());
    println!("{}", a.or(Some(5)).unwrap());
    println!("{}", a.xor(n).unwrap());
    println!("{}", a.xor(Some(1)).is_none());
    println!("{}", a.and(Some(99)).unwrap());
    println!("{}", n.and(Some(99)).is_none());
    match a.ok_or("missing") {
        Ok(v) => println!("ok {}", v),
        Err(e) => println!("err {}", e),
    }
    match n.ok_or("missing") {
        Ok(v) => println!("ok {}", v),
        Err(e) => println!("err {}", e),
    }
    println!("{}", a.map_or(0, double));
    println!("{}", n.map_or(100, double));
    let mut m = Some(4);
    if let Some(r) = m.as_mut() {
        *r += 1;
    }
    println!("{}", m.unwrap());
    let taken = m.take();
    println!("{} {}", taken.unwrap(), m.is_none());
    let old = m.replace(8);
    println!("{} {}", old.is_none(), m.unwrap());
    let r = m.get_or_insert(50);
    *r += 1;
    println!("{}", m.unwrap());
    let mut e: Option<i32> = None;
    *e.get_or_insert(50) += 1;
    println!("{}", e.unwrap());
    println!("{}", Some(2).expect("must be there"));
    let z = Some((1, 2)).unwrap_or((0, 0));
    println!("{} {}", z.0, z.1);
    println!("{}", Some(5).is_some_and(is_big2));
    let refd: Option<&i32> = a.as_ref();
    println!("{}", *refd.unwrap());
    let cp: Option<i32> = refd.copied();
    println!("{}", cp.unwrap());
}

fn is_big2(x: i32) -> bool {
    x > 3
}
