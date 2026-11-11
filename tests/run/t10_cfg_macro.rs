// cfg!() with all/any/not, cfg_attr, debug_assert! being compiled out in release.
#[cfg_attr(not(debug_assertions), derive(Debug))]
struct OnlyDebugInRelease {
    v: i32,
}

#[cfg_attr(all(unix, not(test)), derive(Clone, PartialEq))]
#[derive(Debug)]
struct Multi(u8);

fn checked() -> i32 {
    let mut side = 0;
    debug_assert!({ side += 1; false }, "debug_assert must not run in release");
    debug_assert_eq!(1, 2);
    debug_assert_ne!(3, 3);
    side
}

fn main() {
    println!("debug_assertions: {}", cfg!(debug_assertions));
    println!("test: {}", cfg!(test));
    println!("unix: {}", cfg!(unix));
    println!("windows: {}", cfg!(windows));
    println!("not(windows): {}", cfg!(not(windows)));
    println!("any(unix, windows): {}", cfg!(any(unix, windows)));
    println!("all(unix, debug_assertions): {}", cfg!(all(unix, debug_assertions)));
    println!("all(): {}", cfg!(all()));
    println!("any(): {}", cfg!(any()));
    println!("feature=x: {}", cfg!(feature = "x"));
    println!("macos or linux: {}", cfg!(any(target_os = "macos", target_os = "linux")));
    println!("nested: {}", cfg!(not(any(test, all(windows, unix)))));
    let n = if cfg!(debug_assertions) { 1 } else { 2 };
    println!("n = {}", n);
    let o = OnlyDebugInRelease { v: 7 };
    println!("{:?} {}", o, o.v);
    let m = Multi(4);
    let m2 = m.clone();
    println!("{:?} {}", m2, m == m2);
    println!("side effects of debug_assert: {}", checked());
    // overflow-checks are off: plain arithmetic wraps (deliberate, release semantics)
    let x: u8 = 250;
    let y = x + std::hint::black_box(10);
    println!("wrapped {}", y);
}
