// A scoped thread that panics and is not joined makes thread::scope panic
// on the main thread (exit 101) after all scoped threads have finished.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

static DONE: AtomicUsize = AtomicUsize::new(0);

fn main() {
    let data = vec![1, 2, 3];
    println!("start");
    thread::scope(|s| {
        s.spawn(|| {
            DONE.fetch_add(1, Ordering::SeqCst);
            if data.len() == 3 {
                panic!("scoped thread fails");
            }
        });
        s.spawn(|| {
            DONE.fetch_add(1, Ordering::SeqCst);
        });
        println!("spawned");
    });
    println!("unreachable {}", DONE.load(Ordering::SeqCst));
}
