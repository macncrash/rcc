// LazyLock / OnceLock / Once statics initialised exactly once while many
// threads race for them.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Once, OnceLock};
use std::thread;

static LAZY_INITS: AtomicUsize = AtomicUsize::new(0);
static CELL_INITS: AtomicUsize = AtomicUsize::new(0);
static ONCE_RUNS: AtomicUsize = AtomicUsize::new(0);

fn slow() {
    // make the initialiser take a while so that the others really wait
    let mut x = 0u64;
    for i in 0..200_000u64 {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(i);
    }
    if x == 1 {
        println!("unlikely");
    }
    thread::yield_now();
}

static TABLE: LazyLock<Vec<(u32, String)>> = LazyLock::new(|| {
    LAZY_INITS.fetch_add(1, Ordering::SeqCst);
    slow();
    (0..10).map(|i| (i, format!("v{}", i * i))).collect()
});

fn base() -> u64 {
    40
}
static PLAIN: LazyLock<u64> = LazyLock::new(base);

static CONFIG: OnceLock<String> = OnceLock::new();
static INIT: Once = Once::new();

fn config() -> &'static str {
    CONFIG.get_or_init(|| {
        CELL_INITS.fetch_add(1, Ordering::SeqCst);
        slow();
        String::from("configured")
    })
}

fn main() {
    println!("before: inits {} cell {:?} once {}", LAZY_INITS.load(Ordering::SeqCst), CONFIG.get(), INIT.is_completed());
    let seen = Arc::new(AtomicUsize::new(0));
    let handles: Vec<_> = (0..8)
        .map(|t| {
            let seen = seen.clone();
            thread::spawn(move || {
                let v = TABLE.iter().find(|(k, _)| *k == t as u32).map(|(_, v)| v.clone()).unwrap();
                let c = config();
                INIT.call_once(|| {
                    ONCE_RUNS.fetch_add(1, Ordering::SeqCst);
                    slow();
                });
                // after call_once returns the Once is complete in every thread
                if INIT.is_completed() {
                    seen.fetch_add(1, Ordering::SeqCst);
                }
                format!("{} {} {}", t, v, c)
            })
        })
        .collect();
    let mut out: Vec<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    out.sort();
    for line in &out {
        println!("{}", line);
    }
    println!("lazy inits {}", LAZY_INITS.load(Ordering::SeqCst));
    println!("cell inits {}", CELL_INITS.load(Ordering::SeqCst));
    println!("once runs {}", ONCE_RUNS.load(Ordering::SeqCst));
    println!("completed seen {}", seen.load(Ordering::SeqCst));
    println!("table len {} plain {}", TABLE.len(), *PLAIN + 2);
    println!("config {:?} set again {:?}", CONFIG.get(), CONFIG.set(String::from("other")));

    // OnceLock::set racing: exactly one thread wins
    let cell: Arc<OnceLock<usize>> = Arc::new(OnceLock::new());
    let hs: Vec<_> = (0..6)
        .map(|i| {
            let cell = cell.clone();
            thread::spawn(move || cell.set(i).is_ok())
        })
        .collect();
    let wins = hs.into_iter().map(|h| h.join().unwrap()).filter(|w| *w).count();
    println!("set winners {} value present {}", wins, cell.get().is_some());

    let mut local: OnceLock<Vec<u8>> = OnceLock::new();
    println!("local {:?}", local);
    local.get_or_init(|| vec![1, 2]);
    local.get_mut().unwrap().push(3);
    println!("local {:?} into_inner {:?}", local, local.clone().into_inner());

    let lazy = LazyLock::new(|| 6 * 7);
    println!("lazy local {} force {}", *lazy, *LazyLock::force(&lazy));
}
