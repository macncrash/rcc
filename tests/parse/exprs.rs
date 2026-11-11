// Struct literals vs conditions, closures, blocks, statement rules.
fn cond() {
    if x == y { a } else { b }
    if let Some(v) = map.get(&k) { v } else if z { 1 } else { 2 };
    while let Some(top) = stack.pop() { process(top); }
    match s { S { a, .. } => a, _ => 0 }
    for i in 0..n { sum += i; }
    for (i, x) in v.iter().enumerate() {}
    if (Foo { a: 1 }).a == 1 {}
    if x == (Point { x: 1, y: 2 }) {}
    let p = Point { x, y: 2, ..Default::default() };
    let q = Tuple { 0: a, 1: b };
    let s = S {};
    if a.iter().any(|x| *x == T { v: 1 }.v) {}
    match x { _ if (S { a: 1 }).a > 0 => {} _ => {} }
}
fn closures() {
    let f = |x| x + 1;
    let g = |x: i32, y: &mut Vec<u8>| -> i32 { x };
    let h = move || { 42 };
    let k = || ();
    let m = |&(a, b): &(u8, u8)| a + b;
    let n = |_| {};
    let r = v.iter().map(|x| x * 2).filter(|&x| x > 3).collect::<Vec<_>>();
    let c = |a, b| a || b;
    let d = || || 1;
    let e = |x| match x { 1 => 2, _ => 3 };
    s.sort_by(|a, b| b.cmp(a));
    let o = a || b;
}
fn stmts() -> i32 {
    let x;
    let y: u8;
    let Some(z) = opt else { return 0; };
    let Ok(w) = res else { panic!("bad") };
    if a { b() } else { c() }
    match q { _ => {} }
    { inner(); }
    unsafe { ptr.read() };
    loop { break; }
    let v = loop { break 5; };
    'outer: for i in 0..10 {
        'inner: while true {
            if i == 3 { continue 'outer; }
            break 'inner;
        }
    }
    let lb = 'blk: { if c { break 'blk 1; } 2 };
    if c { 1 } else { 2 }.to_string();
    match m { _ => 1 }.max(2);
    vec![1, 2].len();
    println!("{}", x);
    my_macro! { a b c }
    thread_local!(static X: u8 = 0);
    *x.borrow_mut() = 5;
    (a, b) = (b, a);
    _ = f();
    [x, y] = arr;
    S { a, .. } = s;
    let t = (1).max(2);
    let neg = -1i32.pow(2);
    return x?;
    x
}
