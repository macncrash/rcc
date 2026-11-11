use std::cell::RefCell;
use std::rc::Rc;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let a = Rc::new(Loud("a"));
    let a2 = a.clone();
    println!("count={}", Rc::strong_count(&a));
    drop(a);
    println!("dropped first handle, count={}", Rc::strong_count(&a2));
    drop(a2);
    println!("after last handle");

    let shared = Rc::new(RefCell::new(vec![Loud("v1"), Loud("v2")]));
    let s2 = shared.clone();
    s2.borrow_mut().push(Loud("v3"));
    println!("pop one");
    s2.borrow_mut().remove(0);
    drop(shared);
    println!("one handle left, len={}", s2.borrow().len());
    drop(s2);
    println!("vec gone");

    let w;
    {
        let inner = Rc::new(Loud("scoped"));
        w = Rc::downgrade(&inner);
        println!("upgrade ok={}", w.upgrade().is_some());
    }
    println!("upgrade after scope ok={}", w.upgrade().is_some());

    let cell = RefCell::new(Some(Loud("cell")));
    let old = cell.replace(Some(Loud("cell2")));
    println!("replaced");
    drop(old);
    *cell.borrow_mut() = None;
    println!("set None");
    println!("end of main");
}
