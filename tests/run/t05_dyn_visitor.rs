// Struct holding Box<dyn Trait> fields; visitor pattern over an enum AST with
// a dyn visitor; trait object passed through several layers.
enum Expr {
    Num(i64),
    Neg(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}

trait Visitor {
    fn num(&mut self, n: i64);
    fn enter(&mut self, op: &str);
    fn leave(&mut self, op: &str);
}

fn walk(e: &Expr, v: &mut dyn Visitor) {
    match e {
        Expr::Num(n) => v.num(*n),
        Expr::Neg(a) => {
            v.enter("neg");
            walk(a, v);
            v.leave("neg");
        }
        Expr::Add(a, b) => {
            v.enter("add");
            walk(a, v);
            walk(b, v);
            v.leave("add");
        }
        Expr::Mul(a, b) => {
            v.enter("mul");
            walk(a, v);
            walk(b, v);
            v.leave("mul");
        }
    }
}

struct Printer {
    out: String,
}
impl Visitor for Printer {
    fn num(&mut self, n: i64) {
        self.out.push_str(&format!(" {}", n));
    }
    fn enter(&mut self, op: &str) {
        self.out.push_str(&format!(" ({}", op));
    }
    fn leave(&mut self, _op: &str) {
        self.out.push(')');
    }
}

struct Evaluator {
    stack: Vec<i64>,
}
impl Visitor for Evaluator {
    fn num(&mut self, n: i64) {
        self.stack.push(n);
    }
    fn enter(&mut self, _op: &str) {}
    fn leave(&mut self, op: &str) {
        let b = self.stack.pop().unwrap();
        let r = match op {
            "neg" => -b,
            "add" => self.stack.pop().unwrap() + b,
            "mul" => self.stack.pop().unwrap() * b,
            _ => unreachable!(),
        };
        self.stack.push(r);
    }
}

struct Counter {
    nodes: u32,
    max_depth: u32,
    depth: u32,
}
impl Visitor for Counter {
    fn num(&mut self, _n: i64) {
        self.nodes += 1;
    }
    fn enter(&mut self, _op: &str) {
        self.nodes += 1;
        self.depth += 1;
        if self.depth > self.max_depth {
            self.max_depth = self.depth;
        }
    }
    fn leave(&mut self, _op: &str) {
        self.depth -= 1;
    }
}

struct Session {
    visitors: Vec<(&'static str, Box<dyn Visitor>)>,
}

fn num(n: i64) -> Box<Expr> {
    Box::new(Expr::Num(n))
}

fn main() {
    let e = Expr::Add(num(2), Box::new(Expr::Mul(Box::new(Expr::Neg(num(3))), Box::new(Expr::Add(num(4), num(5))))));
    let mut p = Printer { out: String::new() };
    walk(&e, &mut p);
    println!("{}", p.out);
    let mut ev = Evaluator { stack: vec![] };
    walk(&e, &mut ev);
    println!("{:?}", ev.stack);
    let mut c = Counter { nodes: 0, max_depth: 0, depth: 0 };
    walk(&e, &mut c);
    println!("{} {}", c.nodes, c.max_depth);
    let mut s = Session { visitors: vec![("print", Box::new(Printer { out: String::from(">") })), ("eval", Box::new(Evaluator { stack: vec![100] }))] };
    for (name, v) in s.visitors.iter_mut() {
        walk(&Expr::Mul(num(6), num(7)), v.as_mut());
        println!("visited with {}", name);
    }
    println!("{}", s.visitors.len());
}
