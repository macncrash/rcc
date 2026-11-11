enum Op {
    Add,
    Mul,
    Neg,
}

enum Inner {
    Num(i64),
    Pair(i64, i64),
}

enum Outer {
    One(Inner),
    Two { op: Op, a: Inner, b: Inner },
    Nothing,
}

fn inner_val(i: &Inner) -> i64 {
    match i {
        Inner::Num(n) => *n,
        Inner::Pair(a, b) => a * 10 + b,
    }
}

fn eval(o: &Outer) -> i64 {
    match o {
        Outer::One(Inner::Num(n)) => *n,
        Outer::One(Inner::Pair(a, b)) => a - b,
        Outer::Two { op: Op::Add, a, b } => inner_val(a) + inner_val(b),
        Outer::Two { op: Op::Mul, a, b } => inner_val(a) * inner_val(b),
        Outer::Two { op: Op::Neg, a, .. } => -inner_val(a),
        Outer::Nothing => -1,
    }
}

fn main() {
    let items = [
        Outer::One(Inner::Num(5)),
        Outer::One(Inner::Pair(9, 2)),
        Outer::Two { op: Op::Add, a: Inner::Num(3), b: Inner::Pair(1, 2) },
        Outer::Two { op: Op::Mul, a: Inner::Pair(0, 7), b: Inner::Num(-3) },
        Outer::Two { op: Op::Neg, a: Inner::Num(44), b: Inner::Num(0) },
        Outer::Nothing,
    ];
    let mut acc = 0;
    for i in 0..items.len() {
        let v = eval(&items[i]);
        acc += v * (i as i64 + 1);
        println!("item {} = {} acc {}", i, v, acc);
    }
    if let Outer::Two { b: Inner::Num(z), .. } = &items[4] {
        println!("b of neg {}", z);
    }
}
