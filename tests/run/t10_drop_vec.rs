// Vec of droppables: element order, pop, truncate, clear, remove, insert, assignment,
// drain, retain, and dropping the whole vector.
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let mut v: Vec<D> = (0..8).map(D).collect();
    println!("len {}", v.len());
    let p = v.pop();
    println!("popped {}", p.as_ref().unwrap().0);
    drop(p);
    println!("-- truncate(5)");
    v.truncate(5);
    println!("-- remove(1)");
    let r = v.remove(1);
    println!("removed {}", r.0);
    drop(r);
    println!("-- swap_remove(0)");
    let s = v.swap_remove(0);
    println!("swap_removed {}", s.0);
    drop(s);
    println!("-- index assignment drops old element");
    v[0] = D(100);
    println!("-- insert");
    v.insert(1, D(200));
    print!("contents:");
    for d in &v {
        print!(" {}", d.0);
    }
    println!();
    println!("-- retain odd");
    v.retain(|d| d.0 % 2 == 1);
    println!("-- push and clear");
    v.push(D(300));
    v.push(D(301));
    v.clear();
    println!("cleared len {}", v.len());
    println!("-- drain middle");
    let mut w: Vec<D> = (10..15).map(D).collect();
    let drained: Vec<D> = w.drain(1..3).collect();
    println!("drained {} {}", drained[0].0, drained[1].0);
    drop(drained);
    println!("-- into_iter partially consumed");
    let mut it = w.into_iter();
    let first = it.next().unwrap();
    println!("first {}", first.0);
    drop(it);
    println!("-- nested vec");
    let nested = vec![vec![D(50), D(51)], vec![D(60)]];
    drop(nested);
    println!("-- vec dropped at end of scope");
    let _tail = vec![D(90), D(91), D(92)];
    println!("end of main");
}
