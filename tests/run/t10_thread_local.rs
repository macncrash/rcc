// thread_local! with Cell and RefCell, with/try_with, per-thread state.
use std::cell::{Cell, RefCell};

thread_local! {
    static COUNTER: Cell<u32> = Cell::new(0);
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}
thread_local!(static DEPTH: Cell<usize> = const { Cell::new(10) });

fn bump() -> u32 {
    COUNTER.with(|c| {
        c.set(c.get() + 1);
        c.get()
    })
}

fn log(msg: &str) {
    LOG.with(|l| l.borrow_mut().push(msg.to_string()));
}

fn recurse(n: u32) -> u32 {
    DEPTH.with(|d| d.set(d.get() + 1));
    if n == 0 { 0 } else { n + recurse(n - 1) }
}

fn main() {
    println!("{} {} {}", bump(), bump(), bump());
    log("start");
    log("middle");
    println!("recurse {}", recurse(5));
    println!("depth {}", DEPTH.with(|d| d.get()));
    println!("counter via get {}", COUNTER.with(Cell::get));
    COUNTER.set(100);
    println!("after set {}", COUNTER.get());
    let handle = std::thread::spawn(|| {
        // fresh per-thread values
        let a = bump();
        let b = bump();
        log("in thread");
        let n = LOG.with(|l| l.borrow().len());
        (a, b, n, DEPTH.get())
    });
    let r = handle.join().unwrap();
    println!("thread saw {:?}", r);
    println!("main counter still {}", COUNTER.with(|c| c.get()));
    log("end");
    LOG.with(|l| {
        for (i, m) in l.borrow().iter().enumerate() {
            println!("log[{}] = {}", i, m);
        }
    });
    let taken = LOG.take();
    println!("taken {} now {}", taken.len(), LOG.with(|l| l.borrow().len()));
    println!("try_with ok: {}", COUNTER.try_with(|c| c.get()).is_ok());
}
