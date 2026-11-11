// Arc shared across threads: counts, Weak, try_unwrap, make_mut, get_mut,
// Arc<dyn Trait>, Arc<str>, scoped threads.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::thread;

trait Shape: Send + Sync {
    fn area(&self) -> f64;
    fn name(&self) -> String;
}
struct Sq(f64);
struct Rect(f64, f64);
impl Shape for Sq {
    fn area(&self) -> f64 {
        self.0 * self.0
    }
    fn name(&self) -> String {
        format!("Sq({})", self.0)
    }
}
impl Shape for Rect {
    fn area(&self) -> f64 {
        self.0 * self.1
    }
    fn name(&self) -> String {
        format!("Rect({}, {})", self.0, self.1)
    }
}

static DROPS: AtomicUsize = AtomicUsize::new(0);
struct Tracked(u32);
impl Drop for Tracked {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

fn main() {
    let data = Arc::new(Tracked(5));
    let weak: Weak<Tracked> = Arc::downgrade(&data);
    let hs: Vec<_> = (0..6)
        .map(|i| {
            let d = Arc::clone(&data);
            thread::spawn(move || {
                let local = d.clone();
                local.0 * i
            })
        })
        .collect();
    let r: Vec<u32> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    println!("results {:?}", r);
    println!("strong {} weak {}", Arc::strong_count(&data), Arc::weak_count(&data));
    println!("upgrade {:?}", weak.upgrade().map(|a| a.0));
    drop(data);
    println!("after drop: upgrade none {} drops {}", weak.upgrade().is_none(), DROPS.load(Ordering::SeqCst));

    let shapes: Vec<Arc<dyn Shape>> = vec![Arc::new(Sq(2.0)), Arc::new(Rect(2.0, 3.5))];
    let hs: Vec<_> = shapes
        .iter()
        .map(|s| {
            let s = Arc::clone(s);
            thread::spawn(move || format!("{} {}", s.name(), s.area()))
        })
        .collect();
    for h in hs {
        println!("{}", h.join().unwrap());
    }
    println!("shape counts {:?}", shapes.iter().map(|s| Arc::strong_count(s)).collect::<Vec<_>>());

    let s: Arc<str> = Arc::from("shared text");
    let s2 = s.clone();
    let h = thread::spawn(move || s2.len());
    println!("str len {} eq {} ptr_eq {}", h.join().unwrap(), &*s == "shared text", Arc::ptr_eq(&s, &s.clone()));

    let mut a = Arc::new(vec![1, 2, 3]);
    let b = a.clone();
    Arc::make_mut(&mut a).push(4);
    println!("make_mut {:?} {:?} ptr_eq {}", a, b, Arc::ptr_eq(&a, &b));
    println!("get_mut {:?}", Arc::get_mut(&mut a).map(|v| v.len()));
    let c = a.clone();
    println!("get_mut shared {:?}", Arc::get_mut(&mut a).is_none());
    println!("try_unwrap shared {:?}", Arc::try_unwrap(c).is_err());
    println!("try_unwrap unique {:?}", Arc::try_unwrap(a));

    // scoped threads borrow local data
    let mut nums = vec![1u64, 2, 3, 4, 5, 6, 7, 8];
    let total = Mutex::new(0u64);
    thread::scope(|sc| {
        for chunk in nums.chunks(3) {
            let total = &total;
            sc.spawn(move || {
                let s: u64 = chunk.iter().sum();
                *total.lock().unwrap() += s;
            });
        }
    });
    println!("scoped total {}", total.into_inner().unwrap());
    let (left, right) = nums.split_at_mut(4);
    thread::scope(|sc| {
        let h1 = sc.spawn(|| {
            for x in left.iter_mut() {
                *x *= 10;
            }
            left.len()
        });
        let h2 = sc.spawn(|| {
            for x in right.iter_mut() {
                *x += 1;
            }
            right.len()
        });
        println!("scoped joins {} {}", h1.join().unwrap(), h2.join().unwrap());
    });
    println!("nums {:?}", nums);

    // Condvar handshake
    let pair = Arc::new((Mutex::new(0u32), Condvar::new()));
    let p2 = pair.clone();
    let h = thread::spawn(move || {
        let (m, cv) = &*p2;
        for _ in 0..3 {
            *m.lock().unwrap() += 1;
            cv.notify_all();
        }
        "notified"
    });
    {
        let (m, cv) = &*pair;
        let g = cv.wait_while(m.lock().unwrap(), |v| *v < 3).unwrap();
        println!("condvar value {}", *g);
    }
    println!("{}", h.join().unwrap());
    println!("final pair count {}", Arc::strong_count(&pair));
}
