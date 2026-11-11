use std::cell::{Cell, RefCell};

struct Counter {
    hits: Cell<u32>,
    log: RefCell<Vec<String>>,
}

impl Counter {
    fn touch(&self, what: &str) -> u32 {
        self.hits.set(self.hits.get() + 1);
        self.log.borrow_mut().push(format!("{}#{}", what, self.hits.get()));
        self.hits.get()
    }
}

fn main() {
    let c = Cell::new(5);
    c.set(c.get() * 2);
    println!("cell={}", c.get());
    let old = c.replace(42);
    println!("replace old={} new={}", old, c.get());
    let taken = c.take();
    println!("take={} now={}", taken, c.get());
    let a = Cell::new(1);
    let b = Cell::new(2);
    a.swap(&b);
    println!("swap a={} b={}", a.get(), b.get());
    println!("into_inner={}", a.into_inner());

    let k = Counter { hits: Cell::new(0), log: RefCell::new(vec![]) };
    for w in ["x", "y", "z"] {
        let n = k.touch(w);
        println!("touch {} -> {}", w, n);
    }
    println!("log={:?}", k.log.borrow());

    let r = RefCell::new(vec![1, 2, 3]);
    {
        let b1 = r.borrow();
        let b2 = r.borrow();
        println!("two shared borrows: {} {}", b1.len(), b2[0]);
        println!("try_borrow_mut while shared is_err={}", r.try_borrow_mut().is_err());
    }
    {
        let mut m = r.borrow_mut();
        m.push(4);
        m[0] = 100;
        println!("try_borrow while mut is_err={}", r.try_borrow().is_err());
    }
    println!("after: {:?}", r.borrow());
    let old = r.replace(vec![9]);
    println!("replaced old={:?} now={:?}", old, r.borrow());
    r.borrow_mut().extend_from_slice(&[8, 7]);
    let new_len = {
        let mut g = r.borrow_mut();
        g.sort();
        g.len()
    };
    println!("sorted len={} {:?}", new_len, r.borrow());
    let v = r.replace_with(|old| old.iter().map(|x| x * 10).collect());
    println!("replace_with old={:?} new={:?}", v, r.borrow());
    let inner = r.into_inner();
    println!("into_inner {:?}", inner);
    let cells: Vec<Cell<i32>> = (0..5).map(Cell::new).collect();
    for i in 1..cells.len() {
        cells[i].set(cells[i].get() + cells[i - 1].get());
    }
    println!("prefix sums {:?}", cells.iter().map(|c| c.get()).collect::<Vec<_>>());
    let rc = RefCell::new(String::from("abc"));
    rc.borrow_mut().push_str("def");
    let len = rc.borrow().len();
    println!("{} {}", rc.borrow(), len);
}
