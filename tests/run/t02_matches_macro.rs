enum Kind {
    Ident,
    Number,
    Plus,
    Minus,
    Star,
    Eof,
}

fn is_op(k: &Kind) -> bool {
    matches!(k, Kind::Plus | Kind::Minus | Kind::Star)
}

fn name(k: &Kind) -> &'static str {
    match k {
        Kind::Ident => "ident",
        Kind::Number => "number",
        Kind::Plus => "plus",
        Kind::Minus => "minus",
        Kind::Star => "star",
        Kind::Eof => "eof",
    }
}

fn main() {
    let ks = [Kind::Ident, Kind::Plus, Kind::Number, Kind::Star, Kind::Minus, Kind::Eof];
    let mut ops = 0;
    for i in 0..ks.len() {
        let k = &ks[i];
        let op = is_op(k);
        if op {
            ops += 1;
        }
        println!("{} op={} term={} eof={}", name(k), op,
            matches!(k, Kind::Ident | Kind::Number), matches!(*k, Kind::Eof));
    }
    println!("ops {}", ops);
    for n in 0..12 {
        let small_even = matches!(n, x if x % 2 == 0 && x < 7);
        let in_range = matches!(n, 3..=8);
        println!("{} {} {}", n, small_even, in_range);
    }
    let c = 'k';
    println!("{} {}", matches!(c, 'a'..='z'), matches!(c, 'A'..='Z' | '0'..='9'));
    let t = (1, 2);
    println!("{}", matches!(t, (1, _)));
}
