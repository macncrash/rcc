enum Value {
    Int(i64),
    Bool(bool),
    Pair(i64, i64),
}

fn as_int(v: &Value) -> i64 {
    let Value::Int(n) = v else {
        println!("not an int");
        return -1;
    };
    *n * 10
}

fn sum_pair(v: &Value) -> Option<i64> {
    let Value::Pair(a, b) = *v else { return None };
    Some(a + b)
}

fn first_true(vs: &[Value; 4]) -> usize {
    let mut i = 0;
    loop {
        if i == vs.len() {
            return 99;
        }
        let Value::Bool(true) = vs[i] else {
            i += 1;
            continue;
        };
        return i;
    }
}

fn digit(c: char) -> u32 {
    let d @ '0'..='9' = c else {
        return 100;
    };
    d as u32 - '0' as u32
}

fn main() {
    let vs = [Value::Int(4), Value::Bool(false), Value::Pair(3, 8), Value::Bool(true)];
    for i in 0..vs.len() {
        println!("as_int {}", as_int(&vs[i]));
        match sum_pair(&vs[i]) {
            Some(s) => println!("pair sum {}", s),
            None => println!("no pair"),
        }
    }
    println!("first true {}", first_true(&vs));
    println!("{} {} {}", digit('0'), digit('7'), digit('x'));
    let (a, b) = (1, 2);
    let 1 = a else { panic!() };
    println!("b {}", b);
}
