// Moved values are dropped by their new owner, not the original binding.
struct D(String);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn consume(d: D) {
    println!("consume {}", d.0);
}

fn pass_through(d: D) -> D {
    println!("pass {}", d.0);
    d
}

struct Holder {
    item: D,
}

fn conditional(flag: bool) {
    let d = D(format!("cond-{}", flag));
    if flag {
        consume(d);
        println!("moved in branch");
    } else {
        println!("not moved");
    }
    // drop flag decides whether d is dropped here
    println!("end of conditional({})", flag);
}

fn main() {
    let a = D("a".to_string());
    consume(a);
    println!("after consume");
    let b = D("b".to_string());
    let b2 = pass_through(b);
    println!("after pass_through {}", b2.0);
    let h = Holder { item: D("held".to_string()) };
    println!("holder created");
    let inner = h.item; // moves the whole field out; h has no drop glue left
    println!("moved out {}", inner.0);
    conditional(true);
    conditional(false);
    let v = vec![D("v0".to_string()), D("v1".to_string())];
    let w = v; // move of the vector itself
    println!("vec moved, len {}", w.len());
    let mut opt = Some(D("opt".to_string()));
    if let Some(d) = opt.take() {
        println!("took {}", d.0);
    }
    println!("opt is_none {}", opt.is_none());
    let boxed = Box::new(D("boxed".to_string()));
    let unboxed = *boxed;
    println!("unboxed {}", unboxed.0);
    drop(b2);
    println!("end of main");
}
