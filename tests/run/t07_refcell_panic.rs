// A double mutable borrow of a RefCell must panic (exit code 101).
use std::cell::RefCell;

fn main() {
    let r = RefCell::new(10);
    {
        let mut g = r.borrow_mut();
        *g += 1;
        println!("first borrow ok: {}", *g);
    }
    println!("released, value={}", r.borrow());
    let _g1 = r.borrow_mut();
    println!("holding a mutable borrow");
    let _g2 = r.borrow_mut();
    println!("unreachable");
}
