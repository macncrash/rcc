enum Expr {
    Num(i64),
    Var,
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
}

fn eval(e: &Expr, x: i64) -> i64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Var => x,
        Expr::Add(a, b) => eval(a, x) + eval(b, x),
        Expr::Mul(a, b) => eval(a, x) * eval(b, x),
        Expr::Neg(a) => -eval(a, x),
    }
}

fn print(e: &Expr) {
    match e {
        Expr::Num(n) => print!("{}", n),
        Expr::Var => print!("x"),
        Expr::Add(a, b) => {
            print!("(");
            print(a);
            print!(" + ");
            print(b);
            print!(")");
        }
        Expr::Mul(a, b) => {
            print(a);
            print!("*");
            print(b);
        }
        Expr::Neg(a) => {
            print!("-");
            print(a);
        }
    }
}

fn simplify(e: Expr) -> Expr {
    match e {
        Expr::Add(a, b) => match (simplify(*a), simplify(*b)) {
            (Expr::Num(0), r) => r,
            (l, Expr::Num(0)) => l,
            (Expr::Num(x), Expr::Num(y)) => Expr::Num(x + y),
            (l, r) => Expr::Add(Box::new(l), Box::new(r)),
        },
        Expr::Mul(a, b) => match (simplify(*a), simplify(*b)) {
            (Expr::Num(0), _) | (_, Expr::Num(0)) => Expr::Num(0),
            (Expr::Num(1), r) => r,
            (l, Expr::Num(1)) => l,
            (Expr::Num(x), Expr::Num(y)) => Expr::Num(x * y),
            (l, r) => Expr::Mul(Box::new(l), Box::new(r)),
        },
        Expr::Neg(a) => match simplify(*a) {
            Expr::Num(n) => Expr::Num(-n),
            Expr::Neg(inner) => *inner,
            other => Expr::Neg(Box::new(other)),
        },
        other => other,
    }
}

fn size(e: &Expr) -> u32 {
    match e {
        Expr::Num(_) | Expr::Var => 1,
        Expr::Add(a, b) | Expr::Mul(a, b) => 1 + size(a) + size(b),
        Expr::Neg(a) => 1 + size(a),
    }
}

fn n(v: i64) -> Box<Expr> {
    Box::new(Expr::Num(v))
}

fn main() {
    let e = Expr::Add(
        Box::new(Expr::Mul(n(1), Box::new(Expr::Var))),
        Box::new(Expr::Add(
            Box::new(Expr::Mul(n(0), Box::new(Expr::Var))),
            Box::new(Expr::Neg(Box::new(Expr::Neg(Box::new(Expr::Add(n(2), n(3))))))),
        )),
    );
    print(&e);
    println!();
    println!("size {}", size(&e));
    for x in 0..4 {
        println!("e({}) = {}", x, eval(&e, x));
    }
    let s = simplify(e);
    print(&s);
    println!();
    println!("size {}", size(&s));
    for x in 0..4 {
        println!("s({}) = {}", x, eval(&s, x));
    }
}
