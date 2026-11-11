// thread_local! values differ per thread, are lazily initialised once per
// thread, and are dropped when a spawned thread exits.
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

static INITS: AtomicUsize = AtomicUsize::new(0);

struct Noisy(u32);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("dropping Noisy({})", self.0);
    }
}

fn make_id() -> u32 {
    INITS.fetch_add(1, Ordering::SeqCst) as u32 + 100
}

thread_local! {
    static ID: u32 = make_id();
    static DEPTH: Cell<u32> = const { Cell::new(0) };
    static NAMES: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static NOISY: RefCell<Option<Noisy>> = RefCell::new(None);
}

fn work(tag: &str, n: u32) -> (u32, Vec<String>) {
    for i in 0..n {
        DEPTH.with(|d| d.set(d.get() + i));
        NAMES.with_borrow_mut(|v| v.push(format!("{}{}", tag, i)));
    }
    let d = DEPTH.get();
    let names = NAMES.with_borrow(|v| v.clone());
    (d, names)
}

fn main() {
    println!("inits before {}", INITS.load(Ordering::SeqCst));
    let main_id = ID.with(|x| *x);
    println!("main id {} again {}", main_id, ID.with(|x| *x));
    println!("inits after main {}", INITS.load(Ordering::SeqCst));
    println!("main work {:?}", work("m", 3));

    let mut handles = Vec::new();
    for t in 0..4u32 {
        handles.push(thread::spawn(move || {
            let (d, names) = work("t", t + 1);
            // ID is initialised in each thread separately
            let id = ID.with(|x| *x);
            DEPTH.set(DEPTH.get() * 10);
            (t, d, names.len(), DEPTH.get(), id >= 100)
        }));
    }
    for h in handles {
        println!("thread {:?}", h.join().unwrap());
    }
    println!("inits after threads {}", INITS.load(Ordering::SeqCst));
    println!("main depth unchanged {}", DEPTH.get());
    println!("main names {:?}", NAMES.take());
    println!("main names after take {:?}", NAMES.with_borrow(|v| v.len()));
    DEPTH.set(42);
    println!("replace {} -> {}", DEPTH.replace(7), DEPTH.get());

    // a thread-local with a destructor in a spawned thread
    let h = thread::spawn(|| {
        NOISY.set(Some(Noisy(9)));
        NOISY.with_borrow(|n| n.as_ref().map(|x| x.0))
    });
    let r = h.join().unwrap();
    println!("noisy thread returned {:?}", r);
    println!("main noisy {:?}", NOISY.with_borrow(|n| n.is_none()));
}
