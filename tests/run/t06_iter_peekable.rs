#[derive(Debug)]
enum Tok {
    Num(i64),
    Ident(String),
    Op(char),
}
fn lex(s: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut it = s.chars().peekable();
    while let Some(&c) = it.peek() {
        if c.is_ascii_digit() {
            let mut n = 0i64;
            while let Some(&d) = it.peek() {
                if let Some(v) = d.to_digit(10) {
                    n = n * 10 + v as i64;
                    it.next();
                } else {
                    break;
                }
            }
            out.push(Tok::Num(n));
        } else if c.is_alphabetic() {
            let mut id = String::new();
            while let Some(d) = it.next_if(|d| d.is_alphanumeric()) {
                id.push(d);
            }
            out.push(Tok::Ident(id));
        } else if c == ' ' {
            it.next();
        } else {
            out.push(Tok::Op(c));
            it.next();
        }
    }
    out
}
fn weight(t: &Tok) -> i64 {
    match t {
        Tok::Num(n) => *n,
        Tok::Ident(s) => s.len() as i64,
        Tok::Op(c) => *c as i64,
    }
}
fn main() {
    for t in lex("x1 = 42 + foo*7") {
        println!("{:?} w={}", t, weight(&t));
    }
    let mut p = [1, 1, 2, 2, 2, 3, 1].iter().peekable();
    let mut runs = Vec::new();
    while let Some(&x) = p.next() {
        let mut n = 1;
        while p.next_if_eq(&&x).is_some() {
            n += 1;
        }
        runs.push((x, n));
    }
    println!("{:?}", runs);
    let mut q = (1..4).peekable();
    let p1 = q.peek().copied();
    let p2 = q.peek().copied();
    let n1 = q.next();
    println!("{:?} {:?} {:?}", p1, p2, n1);
    if let Some(x) = q.peek_mut() {
        *x *= 100;
    }
    println!("{:?}", q.collect::<Vec<_>>());
}
