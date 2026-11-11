struct Span {
    lo: u32,
    hi: u32,
}

struct Token {
    kind: u8,
    span: Span,
    neg: bool,
}

fn len(t: &Token) -> u32 {
    let Token { span: Span { lo, hi }, .. } = t;
    hi - lo
}

fn classify(t: &Token) -> &'static str {
    match t {
        Token { kind: 0, .. } => "kind0",
        Token { kind: 1..=3, neg: true, .. } => "small neg",
        Token { kind: 1..=3, neg: false, .. } => "small pos",
        Token { span: Span { lo: 0, .. }, .. } => "at start",
        Token { kind, span: Span { hi, .. }, .. } if *hi > 100 => {
            if *kind > 200 { "far big" } else { "far" }
        }
        _ => "other",
    }
}

fn main() {
    let toks = [
        Token { kind: 0, span: Span { lo: 5, hi: 9 }, neg: false },
        Token { kind: 2, span: Span { lo: 1, hi: 2 }, neg: true },
        Token { kind: 3, span: Span { lo: 1, hi: 4 }, neg: false },
        Token { kind: 9, span: Span { lo: 0, hi: 40 }, neg: false },
        Token { kind: 250, span: Span { lo: 50, hi: 150 }, neg: true },
        Token { kind: 20, span: Span { lo: 50, hi: 150 }, neg: false },
        Token { kind: 20, span: Span { lo: 50, hi: 60 }, neg: false },
    ];
    for i in 0..toks.len() {
        println!("{} len {} {}", i, len(&toks[i]), classify(&toks[i]));
    }
    let Token { kind: k, neg, .. } = &toks[1];
    println!("{} {}", k, neg);
}
