struct P {
    a: i32,
    b: i32,
}

fn main() {
    let x = 5;
    let r = &x;
    match r {
        &5 => println!("ref five"),
        _ => println!("other"),
    }
    match *r {
        5 => println!("deref five"),
        _ => println!("other"),
    }
    let rr = &&7;
    match rr {
        &&n => println!("double ref {}", n),
    }
    let p = P { a: 1, b: 2 };
    let pr = &p;
    let P { a, b } = pr;
    println!("auto-deref binding {} {}", a, b);
    let &P { a: aa, b: bb } = pr;
    println!("explicit {} {}", aa, bb);
    let mut q = P { a: 10, b: 20 };
    {
        let P { a, b } = &mut q;
        *a += 1;
        *b *= 2;
    }
    println!("mutated {} {}", q.a, q.b);
    match &mut q {
        P { a, .. } if *a > 5 => *a = 0,
        _ => {}
    }
    println!("after {} {}", q.a, q.b);
    let pairs = [(1, 'a'), (2, 'b'), (3, 'c')];
    for i in 0..pairs.len() {
        let &(n, c) = &pairs[i];
        let (m, d) = &pairs[i];
        println!("{} {} {} {}", n, c, *m * 2, *d);
    }
    let t = &(1, (2, 3));
    let (one, (two, three)) = t;
    println!("{}", one + two + three);
    let opt: &Option<i32> = &Some(9);
    if let Some(v) = opt {
        println!("opt {}", v + 1);
    }
    let mut o = Some(3);
    if let Some(ref mut v) = o {
        *v += 100;
    }
    if let Some(v) = &mut o {
        *v += 1000;
    }
    match o {
        Some(v) => println!("o {}", v),
        None => println!("none"),
    }
}
