// Patterns of every shape.
fn pats(x: (u8, u8), &(a, b): &(i32, i32), mut m: u8, _: u8) {
    match v {
        0 => {}
        1 | 2 | 3 => {}
        4..=9 => {}
        10..20 => {}
        21.. => {}
        ..=-1 => {}
        'a'..='z' | 'A'..='Z' => {}
        -5 => {}
        b'x' => {}
        "str" => {}
        true | false => {}
        MIN..=MAX => {}
        consts::A..=consts::B => {}
        x @ 1..=5 => {}
        ref r => {}
        ref mut rm => {}
        mut mv => {}
        Some(ref mut inner @ Some(_)) => {}
        Some(Ok(x) | Err(x)) => {}
        Foo::Bar { a, b: (c, d), ref e, mut f, .. } => {}
        Foo::Baz(_, .., z) => {}
        Foo::Unit => {}
        None => {}
        (first, ..) => {}
        (.., last) => {}
        [] => {}
        [one] => {}
        [head, tail @ ..] => {}
        [a, .., b] => {}
        [x, rest @ ..] if x > 0 => {}
        &[ref s, ..] => {}
        &mut Some(ref q) => {}
        &&c => {}
        (y) => {}
        ((a | b), (c | d)) => {}
        <T as Trait>::CONST => {}
        Self::A | Self::B => {}
        S { 0: a, 1: b } => {}
        m!(x) => {}
        | P | Q => {}
        _ => {}
    }
    let (a, mut b) = t;
    let Point { x, y: py } = p;
    for &(k, ref v) in map.iter() {}
    if let Some(1..=5) | None = o {}
}
