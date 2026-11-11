// mem::drop, mem::replace, mem::take, mem::swap, mem::forget, Option::take.
use std::mem;

#[derive(Default)]
struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {:?}", self.0);
    }
}

fn main() {
    let a = D("a");
    println!("before drop(a)");
    drop(a);
    println!("after drop(a)");
    let mut slot = D("first");
    let old = mem::replace(&mut slot, D("second"));
    println!("replaced: old={} slot={}", old.0, slot.0);
    drop(old);
    let taken = mem::take(&mut slot);
    println!("taken={} slot={:?}", taken.0, slot.0);
    let mut x = D("x");
    let mut y = D("y");
    mem::swap(&mut x, &mut y);
    println!("swapped x={} y={}", x.0, y.0);
    let f = D("forgotten");
    mem::forget(f);
    println!("after forget");
    let mut o = Some(D("in-option"));
    println!("take -> {}", o.take().map(|d| d.0).unwrap_or("none"));
    println!("o is_none={}", o.is_none());
    o = Some(D("refilled"));
    let r = o.replace(D("replacement"));
    println!("replace returned {}", r.as_ref().map(|d| d.0).unwrap_or("none"));
    drop(r);
    println!("size_of D = {}", mem::size_of::<D>() == mem::size_of::<&str>());
    let v = vec![D("v1"), D("v2")];
    mem::drop(v);
    println!("end of main");
}
