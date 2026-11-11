// static LazyLock, OnceLock, Once, const fn initialisers, static arrays.
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, Once, OnceLock};

static TABLE: LazyLock<HashMap<&'static str, u32>> = LazyLock::new(|| {
    println!("initialising TABLE");
    let mut m = HashMap::new();
    m.insert("add", 1);
    m.insert("sub", 2);
    m.insert("mul", 3);
    m
});

static CONFIG: OnceLock<String> = OnceLock::new();
static INIT: Once = Once::new();
static LOG: Mutex<Vec<u32>> = Mutex::new(Vec::new());

const fn cube(x: u64) -> u64 {
    x * x * x
}
static CUBES: [u64; 5] = [cube(0), cube(1), cube(2), cube(3), cube(4)];
const NAMES: &[&str] = &["zero", "one", "two"];

fn config() -> &'static str {
    CONFIG.get_or_init(|| {
        println!("initialising CONFIG");
        format!("cfg-{}", 42)
    })
}

fn setup() {
    INIT.call_once(|| {
        println!("Once body runs exactly once");
        LOG.lock().unwrap().push(1);
    });
}

fn main() {
    println!("before first use");
    println!("mul = {}", TABLE["mul"]);
    println!("add = {}", TABLE.get("add").copied().unwrap_or(0));
    println!("len = {}", TABLE.len());
    println!("config = {}", config());
    println!("config again = {}", config());
    println!("CONFIG.get = {:?}", CONFIG.get());
    let other: OnceLock<i32> = OnceLock::new();
    println!("empty get = {:?}", other.get());
    println!("set = {:?}", other.set(5));
    println!("second set = {:?}", other.set(6));
    println!("value = {:?}", other.get());
    println!("completed before: {}", INIT.is_completed());
    setup();
    setup();
    setup();
    println!("completed after: {}", INIT.is_completed());
    LOG.lock().unwrap().push(2);
    println!("log = {:?}", *LOG.lock().unwrap());
    println!("cubes = {:?}", CUBES);
    println!("names = {:?} {}", NAMES, NAMES[2]);
}
