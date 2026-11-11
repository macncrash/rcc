// Several threads increment atomics and push into a Mutex<Vec<_>>.
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

static HITS: AtomicU64 = AtomicU64::new(0);
static MAXV: AtomicI32 = AtomicI32::new(i32::MIN);
static LOG: Mutex<Vec<u32>> = Mutex::new(Vec::new());

fn main() {
    let counter = Arc::new(AtomicUsize::new(0));
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    for t in 0..8u32 {
        let counter = Arc::clone(&counter);
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            for i in 0..10_000u32 {
                counter.fetch_add(1, Ordering::Relaxed);
                HITS.fetch_add(i as u64, Ordering::SeqCst);
                if i % 1000 == 0 {
                    shared.lock().unwrap().push(t * 100 + i / 1000);
                }
            }
            MAXV.fetch_max((t as i32) * 7 - 20, Ordering::SeqCst);
            LOG.lock().unwrap().push(t);
            t * 2
        }));
    }
    let rets: Vec<u32> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    println!("returns {:?}", rets);
    println!("counter {}", counter.load(Ordering::SeqCst));
    println!("hits {}", HITS.load(Ordering::SeqCst));
    println!("max {}", MAXV.load(Ordering::SeqCst));
    let mut v = shared.lock().unwrap().clone();
    println!("pushed {}", v.len());
    v.sort();
    println!("first {:?} last {:?}", &v[..3], &v[v.len() - 3..]);
    let mut log = LOG.lock().unwrap();
    log.sort();
    println!("log {:?}", *log);
    drop(log);
    println!("arc counts {} {}", Arc::strong_count(&counter), Arc::strong_count(&shared));

    // compare_exchange loop as a spin lock between threads
    let lock = Arc::new(AtomicBool::new(false));
    let total = Arc::new(Mutex::new(0u64));
    let hs: Vec<_> = (0..4)
        .map(|k| {
            let lock = lock.clone();
            let total = total.clone();
            thread::spawn(move || {
                for _ in 0..500 {
                    while lock.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
                        std::hint::spin_loop();
                    }
                    *total.lock().unwrap() += k + 1;
                    lock.store(false, Ordering::Release);
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    println!("spin total {}", *total.lock().unwrap());

    // small atomics wrap
    let b = AtomicU8::new(250);
    for _ in 0..10 {
        b.fetch_add(1, Ordering::Relaxed);
    }
    println!("u8 wrapped {}", b.load(Ordering::Relaxed));

    // try_lock while held, into_inner, get_mut
    let m = Mutex::new(5);
    {
        let g = m.lock().unwrap();
        println!("try_lock while held: {}", m.try_lock().is_err());
        println!("guard {}", *g);
    }
    println!("try_lock free: {}", m.try_lock().is_ok());
    let mut m = m;
    *m.get_mut().unwrap() += 1;
    println!("into_inner {}", m.into_inner().unwrap());
    println!("{:?}", Mutex::new(vec![1, 2]));

    // RwLock: readers in parallel, then a writer
    let rw = Arc::new(RwLock::new(vec![1u32, 2, 3]));
    let readers: Vec<_> = (0..4)
        .map(|_| {
            let rw = rw.clone();
            thread::spawn(move || rw.read().unwrap().iter().sum::<u32>())
        })
        .collect();
    let sums: Vec<u32> = readers.into_iter().map(|h| h.join().unwrap()).collect();
    println!("reader sums {:?}", sums);
    rw.write().unwrap().push(4);
    {
        let r1 = rw.read().unwrap();
        let r2 = rw.read().unwrap();
        println!("two readers {} {} try_write {}", r1.len(), r2.len(), rw.try_write().is_err());
    }
    println!("rw {:?}", *rw.read().unwrap());
}
