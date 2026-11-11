// Drops of locals that are not initialized on every path must not run:
// let-else failure paths, early returns, deferred initialization, loops.
use std::cell::RefCell;
thread_local!{}
struct Noisy(&'static str, &'static RefCell<Vec<String>>);
impl Drop for Noisy {
    fn drop(&mut self) { self.1.borrow_mut().push(format!("drop {}", self.0)); }
}
#[inline(never)]
fn dirty(n: u64) -> u64 {
    let mut a = [0xdead_beef_dead_beefu64; 64];
    for i in 0..64 { a[i] ^= n.wrapping_mul(i as u64 + 1); }
    a.iter().fold(0, |x, y| x ^ y)
}
#[derive(Clone)]
struct Def { params: Vec<String>, body: Vec<String> }
#[inline(never)]
fn call(defs: &Vec<(String, Def)>, name: &str, nargs: usize) -> Result<usize, String> {
    let Some(def) = defs.iter().find(|d| d.0 == name).map(|d| d.1.clone()) else {
        return Err(format!("undefined function {}", name));
    };
    if def.params.len() != nargs {
        return Err(format!("{} expects {} args, got {}", name, def.params.len(), nargs));
    }
    let extra;
    if nargs > 1 { extra = def.body.clone(); return Ok(extra.len() + 100); }
    Ok(def.body.len())
}
fn deferred(log: &'static RefCell<Vec<String>>, n: u32) -> u32 {
    let a;
    let b;
    if n == 0 { return 0; }
    a = Noisy("a", log);
    if n == 1 { return 1; }
    b = Noisy("b", log);
    let c = if n == 2 { Noisy("c2", log) } else { let t = Noisy("t", log); drop(t); Noisy("c", log) };
    let _moved = if n == 3 { Some(a) } else { None };
    let _ = &b;
    let _ = &c;
    n
}
fn looped(log: &'static RefCell<Vec<String>>) {
    for i in 0..4 {
        let x;
        if i == 1 { continue; }
        x = Noisy(["x0", "x1", "x2", "x3"][i], log);
        if i == 2 { let y = x; let _ = &y; continue; }
        let Some(_z) = (if i == 3 { None } else { Some(Noisy("z", log)) }) else { break; };
        let _ = &x;
    }
}
fn main() {
    let defs = vec![
        (String::from("f"), Def { params: vec![String::from("x")], body: vec![String::from("s1"), String::from("s2")] }),
        (String::from("g"), Def { params: vec![String::from("x"), String::from("y")], body: vec![String::from("s")] }),
    ];
    let mut junk = 0;
    for round in 0..3 {
        junk ^= dirty(round);
        println!("{:?}", call(&defs, "nope", 1));
        junk ^= dirty(round + 7);
        println!("{:?}", call(&defs, "f", 3));
        println!("{:?}", call(&defs, "f", 1));
        println!("{:?}", call(&defs, "g", 2));
    }
    let _ = junk;
    let log: &'static RefCell<Vec<String>> = Box::leak(Box::new(RefCell::new(Vec::new())));
    for n in 0..5 {
        deferred(log, n);
        println!("{} {:?}", n, log.borrow());
        log.borrow_mut().clear();
    }
    looped(log);
    println!("{:?}", log.borrow());
}
