// Closure captures: by-ref capture does not move; move closures own and drop captures.
struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn call_once<F: FnOnce() -> usize>(f: F) -> usize {
    println!("call_once start");
    let r = f();
    println!("call_once end");
    r
}

// Edition 2021 disjoint capture: `move || d.0.len()` captures only the Copy field d.0,
// so `d` itself stays in the enclosing fn and is dropped when it returns.
fn make_closure() -> impl Fn() -> usize {
    let d = D("returned-capture");
    move || d.0.len()
}

fn main() {
    println!("-- by-ref capture");
    let a = D("a");
    {
        let f = || a.0.len();
        println!("f() = {}", f());
    }
    println!("closure gone, a still alive: {}", a.0);
    println!("-- move capture");
    let b = D("b");
    {
        let g = move || { let whole = &b; whole.0.len() };
        println!("g() = {}", g());
        println!("dropping g");
    }
    println!("after g scope");
    println!("-- FnOnce consumes capture");
    let c = D("c");
    let h = move || {
        let inner = c;
        println!("inside h with {}", inner.0);
        inner.0.len()
    };
    println!("h result {}", call_once(h));
    println!("-- closure never called still drops capture");
    let e = D("e");
    let unused = move || { let whole = &e; whole.0.len() };
    drop(unused);
    println!("after drop(unused)");
    println!("-- returned closure");
    let rc = make_closure();
    println!("rc() = {} {}", rc(), rc());
    drop(rc);
    println!("-- boxed closures in vec");
    let mut fs: Vec<Box<dyn Fn() -> &'static str>> = Vec::new();
    for name in ["k1", "k2"] {
        let d = D(name);
        // captures only d.0 (edition 2021), so d drops at the end of each iteration
        fs.push(Box::new(move || d.0));
    }
    for f in &fs {
        println!("call {}", f());
    }
    drop(fs);
    println!("-- FnMut with captured counter and droppable");
    let mut count = 0;
    let g = D("g");
    let mut inc = || {
        count += 1;
        println!("inc {} with {}", count, g.0);
    };
    inc();
    inc();
    println!("count {}", count);
    println!("end of main");
}
