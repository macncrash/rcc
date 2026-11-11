// Partial moves out of structs and tuples: remaining fields drop at scope end.
struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Pair {
    left: D,
    right: D,
    tag: u32,
}

struct Triple {
    a: D,
    b: D,
    c: D,
}

fn main() {
    println!("-- move one field");
    {
        let p = Pair { left: D("left"), right: D("right"), tag: 7 };
        let l = p.left;
        println!("moved {} tag {}", l.0, p.tag);
        // drops: l (declared later) first, then remaining field p.right
    }
    println!("-- move field into function");
    {
        let t = Triple { a: D("ta"), b: D("tb"), c: D("tc") };
        drop(t.b);
        println!("after drop(t.b), a={}", t.a.0);
    }
    println!("-- destructuring with ..");
    {
        let t = Triple { a: D("da"), b: D("db"), c: D("dc") };
        let Triple { c, .. } = t;
        println!("got {}", c.0);
    }
    println!("-- destructuring all");
    {
        let p = Pair { left: D("pl"), right: D("pr"), tag: 1 };
        let Pair { right, left, tag } = p;
        println!("tag {} {} {}", tag, left.0, right.0);
    }
    println!("-- tuple partial move");
    {
        let tup = (D("t0"), D("t1"), D("t2"));
        let mid = tup.1;
        println!("mid {}", mid.0);
    }
    println!("-- ref binding does not move");
    {
        let p = Pair { left: D("rl"), right: D("rr"), tag: 2 };
        let Pair { ref left, .. } = p;
        println!("borrowed {}", left.0);
    }
    println!("-- reassign moved field");
    {
        let mut p = Pair { left: D("old-left"), right: D("keep-right"), tag: 3 };
        let taken = p.left;
        p.left = D("new-left"); // no drop of old (already moved)
        println!("taken {} now {} tag {}", taken.0, p.left.0, p.tag);
    }
    println!("end");
}
