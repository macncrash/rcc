// Thread names and ids, Builder::name, is_finished, nested spawns, a
// panicking thread next to ones that succeed, join of unit threads.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

fn main() {
    let main = thread::current();
    println!("main name {:?}", main.name());
    let main_id = main.id();
    println!("same id {}", thread::current().id() == main_id);

    let h = thread::Builder::new()
        .name("worker-1".to_string())
        .spawn(move || {
            let me = thread::current();
            let inner = thread::spawn(|| thread::current().name().map(|s| s.to_string()));
            (me.name().map(|s| s.to_string()), me.id() != main_id, inner.join().unwrap())
        })
        .unwrap();
    println!("handle name {:?}", h.thread().name());
    println!("worker {:?}", h.join().unwrap());

    let go = Arc::new(AtomicBool::new(false));
    let g2 = go.clone();
    let h = thread::spawn(move || {
        while !g2.load(Ordering::Acquire) {
            thread::yield_now();
        }
        7
    });
    println!("finished early {}", h.is_finished());
    go.store(true, Ordering::Release);
    while !h.is_finished() {
        thread::yield_now();
    }
    println!("finished later {}", h.is_finished());
    println!("value {}", h.join().unwrap());

    let hs: Vec<_> = (0..6)
        .map(|i| {
            thread::Builder::new()
                .name(format!("w{}", i))
                .spawn(move || {
                    if i % 3 == 1 {
                        panic!("worker {} failed", i);
                    }
                    if i == 5 {
                        panic!("static failure");
                    }
                    i * 11
                })
                .unwrap()
        })
        .collect();
    for (i, h) in hs.into_iter().enumerate() {
        match h.join() {
            Ok(v) => println!("{} ok {}", i, v),
            Err(e) => match e.downcast::<String>() {
                Ok(s) => println!("{} String {}", i, s),
                Err(e) => println!("{} &str {:?}", i, e.downcast_ref::<&str>()),
            },
        }
    }

    // unit-returning threads and a detached thread that is never joined
    let hs: Vec<thread::JoinHandle<()>> = (0..3).map(|_| thread::spawn(|| {})).collect();
    for h in hs {
        h.join().unwrap();
    }
    let flag = Arc::new(AtomicBool::new(false));
    let f2 = flag.clone();
    drop(thread::spawn(move || f2.store(true, Ordering::SeqCst)));
    while !flag.load(Ordering::SeqCst) {
        thread::yield_now();
    }
    println!("detached thread ran");

    // a scoped thread that panics and is joined explicitly
    let r = thread::scope(|s| {
        let h = s.spawn(|| -> u32 { panic!("scoped boom") });
        let ok = s.spawn(|| 5u32);
        (h.join().is_err(), ok.join().unwrap())
    });
    println!("scoped {:?}", r);
}
