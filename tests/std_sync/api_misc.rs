// Less common sync/thread API: AtomicPtr, fetch_nand/fetch_not/fetch_update,
// Arc::new_cyclic/into_inner/unwrap_or_clone, Weak::new, OnceLock::from,
// Once::call_once_force, thread::sleep, LocalKey::try_with, Barrier-like
// handshakes with atomics, panics inside nested spawned threads.
// (Format arguments are variables on purpose: rustc inlines literal
// arguments such as `panic!("{}", 1)` into the format string, which makes
// the payload a &str; rcc's expander does not, see docs/STATUS.md.)
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicPtr, AtomicU16, Ordering};
use std::sync::{Arc, Mutex, Once, OnceLock, Weak};
use std::thread;
use std::time::Duration;

struct Node {
    me: Weak<Node>,
    val: u32,
}

thread_local!(static TL: Cell<u8> = Cell::new(1));

fn main() {
    let mut x = 5i32;
    let mut y = 9i32;
    let p = AtomicPtr::new(&mut x as *mut i32);
    let old = p.swap(&mut y as *mut i32, Ordering::SeqCst);
    println!("ptr swap old {} new {}", unsafe { *old }, unsafe { *p.load(Ordering::SeqCst) });
    println!("ptr cas {}", p.compare_exchange(old, old, Ordering::SeqCst, Ordering::SeqCst).is_err());

    let a = AtomicU16::new(0xF0F0);
    println!("nand {:x} -> {:x}", a.fetch_nand(0xFF00, Ordering::SeqCst), a.load(Ordering::SeqCst));
    let b = AtomicBool::new(true);
    println!("not {} -> {}", b.fetch_not(Ordering::SeqCst), b.load(Ordering::SeqCst));
    println!("bool nand {} -> {}", b.fetch_nand(true, Ordering::SeqCst), b.load(Ordering::SeqCst));
    let c = AtomicI64::new(-5);
    println!("update {:?} {:?}", c.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| Some(v * 2)), c.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| if v > 0 { Some(v) } else { None }));
    println!("min/max {} {} -> {}", c.fetch_min(-100, Ordering::SeqCst), c.fetch_max(3, Ordering::SeqCst), c.load(Ordering::SeqCst));
    println!("debug {:?} {:?}", c, b);

    let n = Arc::new_cyclic(|w| Node { me: w.clone(), val: 77 });
    println!("cyclic {} {}", n.me.upgrade().map(|m| m.val).unwrap_or(0), Arc::weak_count(&n));
    let w: Weak<Node> = Weak::new();
    println!("empty weak {}", w.upgrade().is_none());
    let v = Arc::new(String::from("shared"));
    let v2 = v.clone();
    println!("unwrap_or_clone {}", Arc::unwrap_or_clone(v2));
    println!("into_inner {:?}", Arc::into_inner(v));

    let cell = OnceLock::from(3);
    println!("oncelock from {:?}", cell.get());
    static FORCE: Once = Once::new();
    FORCE.call_once_force(|st| println!("force poisoned {}", st.is_poisoned()));
    FORCE.call_once_force(|_| println!("never"));

    // sleep and ordering between two threads
    let log = Arc::new(Mutex::new(Vec::new()));
    let l2 = log.clone();
    let h = thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        l2.lock().unwrap().push("slept");
    });
    log.lock().unwrap().push("main");
    h.join().unwrap();
    println!("log {:?}", *log.lock().unwrap());

    // a panic in a grandchild thread is seen by the child's join only
    let h = thread::spawn(|| {
        let n = TL.get();
        let inner = thread::spawn(move || -> u8 { panic!("grandchild {}", n) });
        let e = inner.join().unwrap_err();
        let msg = e.downcast_ref::<String>().cloned().unwrap_or_default();
        TL.set(TL.get() + 1);
        (msg, TL.get())
    });
    println!("child saw {:?}", h.join().unwrap());
    println!("main TL {:?}", TL.try_with(|v| v.get()));

    // ping-pong with atomics
    let turn = Arc::new(AtomicBool::new(false));
    let t2 = turn.clone();
    let count = Arc::new(Mutex::new(0u32));
    let c2 = count.clone();
    let h = thread::spawn(move || {
        for _ in 0..100 {
            while !t2.load(Ordering::Acquire) {
                thread::yield_now();
            }
            *c2.lock().unwrap() += 1;
            t2.store(false, Ordering::Release);
        }
    });
    for _ in 0..100 {
        while turn.load(Ordering::Acquire) {
            thread::yield_now();
        }
        *count.lock().unwrap() += 1;
        turn.store(true, Ordering::Release);
    }
    h.join().unwrap();
    println!("ping-pong {}", *count.lock().unwrap());
}
