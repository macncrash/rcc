enum E {
    Neg(i64),
    Zero,
    TooBig,
}

fn check(x: i64) -> Result<i64, E> {
    if x < 0 {
        Err(E::Neg(x))
    } else if x == 0 {
        Err(E::Zero)
    } else if x > 1000 {
        Err(E::TooBig)
    } else {
        Ok(x)
    }
}

fn pipeline(a: i64, b: i64) -> Result<i64, E> {
    let x = check(a)?;
    println!("  a ok {}", x);
    let y = check(b)?;
    println!("  b ok {}", y);
    let z = check(x * y)?;
    println!("  product ok {}", z);
    Ok(z + 1)
}

fn first_even(xs: &[i32]) -> Option<i32> {
    let mut i = 0;
    loop {
        let v = *xs.get(i)?;
        if v % 2 == 0 {
            return Some(v);
        }
        i += 1;
    }
}

fn sum_firsts(a: &[i32], b: &[i32]) -> Option<i32> {
    Some(first_even(a)? + first_even(b)?)
}

fn describe(r: Result<i64, E>) {
    match r {
        Ok(v) => println!("ok {}", v),
        Err(E::Neg(v)) => println!("neg {}", v),
        Err(E::Zero) => println!("zero"),
        Err(E::TooBig) => println!("too big"),
    }
}

fn main() {
    describe(pipeline(3, 4));
    describe(pipeline(-3, 4));
    describe(pipeline(5, 0));
    describe(pipeline(50, 50));
    describe(pipeline(1000, 1));
    let a = [1, 3, 8, 5];
    let b = [7, 9];
    let c = [2];
    println!("{}", first_even(&a).unwrap_or(-1));
    println!("{}", first_even(&b).unwrap_or(-1));
    println!("{}", sum_firsts(&a, &c).unwrap_or(-1));
    println!("{}", sum_firsts(&a, &b).unwrap_or(-1));
}
