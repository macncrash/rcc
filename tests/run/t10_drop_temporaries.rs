// Temporaries: dropped at end of statement, extended by `let x = &temp`, live through
// match / if let scrutinees (edition 2021), tail-expression temporaries.
struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
impl D {
    fn val(&self) -> usize {
        println!("val {}", self.0);
        self.0.len()
    }
    fn opt(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

fn make(name: &'static str) -> D {
    println!("make {}", name);
    D(name)
}

fn sum2(a: usize, b: usize) -> usize {
    println!("sum2");
    a + b
}

fn tail() -> usize {
    let _local = make("tail-local");
    make("tail-temp").val()
}

fn main() {
    println!("-- statement temporaries");
    let n = make("t1").val() + make("t2").val();
    println!("n = {}", n);
    println!("-- temporaries as fn args");
    let s = sum2(make("arg1").val(), make("arg2").val());
    println!("s = {}", s);
    println!("-- lifetime extension");
    let r = &make("extended");
    println!("extended alive: {}", r.0);
    println!("-- match scrutinee temporary lives through the match");
    match make("scrut").opt() {
        Some(k) => println!("in arm k={}", k),
        None => println!("none"),
    }
    println!("after match");
    println!("-- if let scrutinee");
    if let Some(k) = make("iflet").opt() {
        println!("in if-let body {}", k);
    } else {
        println!("else");
    }
    println!("after if let");
    println!("-- if condition temporary is dropped before the body");
    if make("cond").val() > 1 {
        println!("in if body");
    }
    println!("-- while let");
    let mut i = 0;
    while let Some(_) = make("wl").opt() {
        println!("while-let body {}", i);
        i += 1;
        if i == 2 {
            break;
        }
    }
    println!("-- tail expression");
    println!("tail = {}", tail());
    println!("-- method chain on temp");
    let len = make("chain").0.len();
    println!("len = {}", len);
    println!("end of main");
}
