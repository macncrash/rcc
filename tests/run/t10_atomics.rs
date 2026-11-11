// Atomics: load/store/swap/fetch_*/compare_exchange, statics, deterministic threads.
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
static FLAG: AtomicBool = AtomicBool::new(false);

fn next_id() -> usize {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

fn main() {
    println!("ids {} {} {}", next_id(), next_id(), next_id());
    println!("load {}", NEXT_ID.load(Ordering::SeqCst));

    let a = AtomicI64::new(10);
    println!("fetch_add {} -> {}", a.fetch_add(5, Ordering::SeqCst), a.load(Ordering::SeqCst));
    println!("fetch_sub {} -> {}", a.fetch_sub(20, Ordering::SeqCst), a.load(Ordering::SeqCst));
    println!("swap {} -> {}", a.swap(100, Ordering::SeqCst), a.load(Ordering::SeqCst));
    println!("fetch_max {} -> {}", a.fetch_max(150, Ordering::SeqCst), a.load(Ordering::SeqCst));
    println!("fetch_min {} -> {}", a.fetch_min(-3, Ordering::SeqCst), a.load(Ordering::SeqCst));
    a.store(7, Ordering::Release);
    println!("cas ok {:?}", a.compare_exchange(7, 8, Ordering::SeqCst, Ordering::SeqCst));
    println!("cas fail {:?}", a.compare_exchange(7, 9, Ordering::SeqCst, Ordering::SeqCst));
    println!("after cas {}", a.load(Ordering::Acquire));
    let r = a.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| if v < 10 { Some(v * 3) } else { None });
    println!("fetch_update {:?} -> {}", r, a.load(Ordering::SeqCst));
    let r = a.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| if v < 10 { Some(v * 3) } else { None });
    println!("fetch_update {:?}", r);

    let u = AtomicU32::new(0b1100);
    println!("and {} or {} xor {} -> {}", u.fetch_and(0b0110, Ordering::SeqCst),
        u.fetch_or(0b0001, Ordering::SeqCst), u.fetch_xor(0b1111, Ordering::SeqCst), u.load(Ordering::SeqCst));
    // wrapping behaviour of atomics
    let w = AtomicU32::new(u32::MAX);
    w.fetch_add(2, Ordering::SeqCst);
    println!("wrapped {}", w.load(Ordering::SeqCst));
    let mut owned = AtomicUsize::new(3);
    *owned.get_mut() += 4;
    println!("get_mut {} into_inner {}", owned.load(Ordering::SeqCst), owned.into_inner());

    println!("flag {}", FLAG.load(Ordering::SeqCst));
    println!("flag swap {}", FLAG.swap(true, Ordering::SeqCst));
    println!("flag fetch_and {}", FLAG.fetch_and(false, Ordering::SeqCst));
    println!("flag now {}", FLAG.load(Ordering::SeqCst));

    let total = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();
    for t in 0..4 {
        let total = Arc::clone(&total);
        handles.push(std::thread::spawn(move || {
            for i in 0..1000 {
                total.fetch_add(i * (t + 1), Ordering::Relaxed);
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    println!("threaded total {}", total.load(Ordering::SeqCst));
    println!("strong count {}", Arc::strong_count(&total));
}
