// Stmt is only read through derived Debug.
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Num(i64),
    Var(String),
    Neg(Box<Expr>),
    Bin { op: char, lhs: Box<Expr>, rhs: Box<Expr> },
    Call(String, Vec<Expr>),
    Unit,
}

#[derive(Debug)]
enum Stmt<'a> {
    Let(&'a str, Option<Expr>),
    Return(Result<Expr, &'a str>),
    Block(Vec<Stmt<'a>>),
}

fn eval(e: &Expr) -> i64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Var(v) => v.len() as i64,
        Expr::Neg(x) => -eval(x),
        Expr::Bin { op, lhs, rhs } => match op {
            '+' => eval(lhs) + eval(rhs),
            '*' => eval(lhs) * eval(rhs),
            _ => eval(lhs) - eval(rhs),
        },
        Expr::Call(_, args) => args.iter().map(eval).sum(),
        Expr::Unit => 0,
    }
}

fn main() {
    let e = Expr::Bin {
        op: '+',
        lhs: Box::new(Expr::Num(2)),
        rhs: Box::new(Expr::Bin { op: '*', lhs: Box::new(Expr::Var("abc".into())), rhs: Box::new(Expr::Neg(Box::new(Expr::Num(4)))) }),
    };
    println!("{:?}", e);
    println!("{:#?}", e);
    println!("eval={}", eval(&e));
    let c = Expr::Call("f".into(), vec![e.clone(), Expr::Unit, Expr::Num(-1)]);
    println!("{:?}", c);
    println!("eval={} eq={}", eval(&c), c == c.clone());
    let prog = Stmt::Block(vec![
        Stmt::Let("x", Some(Expr::Num(1))),
        Stmt::Let("y", None),
        Stmt::Block(vec![]),
        Stmt::Return(Err("no value")),
        Stmt::Return(Ok(Expr::Var("x".into()))),
    ]);
    println!("{:?}", prog);
    println!("{:#?}", prog);
}
