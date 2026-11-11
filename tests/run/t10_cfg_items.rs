// #[cfg] on items, impls, statements, expressions, struct fields, enum variants, match arms.
// Oracle config: release (debug_assertions off), not test, unix, no features.

#[cfg(debug_assertions)]
fn mode() -> &'static str {
    "debug"
}
#[cfg(not(debug_assertions))]
fn mode() -> &'static str {
    "release"
}

#[cfg(test)]
fn only_in_test() -> i32 {
    compile_error!("must not be compiled")
}

#[cfg(unix)]
mod platform {
    pub fn sep() -> char {
        '/'
    }
}
#[cfg(windows)]
mod platform {
    pub fn sep() -> char {
        '\\'
    }
}

#[cfg(target_os = "macos")]
fn os_family() -> &'static str {
    "unix-like"
}
#[cfg(target_os = "linux")]
fn os_family() -> &'static str {
    "unix-like"
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn os_family() -> &'static str {
    "unix-like"
}

#[derive(Debug)]
struct Settings {
    level: u8,
    #[cfg(feature = "extra")]
    extra: u32,
    #[cfg(not(feature = "extra"))]
    name: &'static str,
}

#[derive(Debug)]
enum Kind {
    A,
    #[cfg(feature = "never")]
    Hidden,
    B(u8),
}

struct Thing;
#[cfg(unix)]
impl Thing {
    fn describe(&self) -> &'static str {
        "unix thing"
    }
}
#[cfg(not(unix))]
impl Thing {
    fn describe(&self) -> &'static str {
        "other thing"
    }
}

fn classify(k: &Kind) -> u32 {
    match k {
        Kind::A => 1,
        #[cfg(feature = "never")]
        Kind::Hidden => 2,
        Kind::B(n) => *n as u32 + 10,
    }
}

fn main() {
    println!("mode {}", mode());
    println!("sep {}", platform::sep());
    println!("family {}", os_family());
    let s = Settings {
        level: 3,
        #[cfg(feature = "extra")]
        extra: 9,
        #[cfg(not(feature = "extra"))]
        name: "basic",
    };
    println!("{:?}", s);
    println!("{} {}", s.level, s.name);
    let ks = [Kind::A, Kind::B(5)];
    for k in &ks {
        println!("{:?} -> {}", k, classify(k));
    }
    println!("{}", Thing.describe());
    let mut total = 0;
    #[cfg(debug_assertions)]
    {
        total += 1000;
    }
    #[cfg(not(debug_assertions))]
    {
        total += 1;
    }
    #[cfg(test)]
    {
        total += only_in_test();
    }
    #[cfg(unix)]
    let label = "unix-label";
    #[cfg(not(unix))]
    let label = "other-label";
    println!("total {} label {}", total, label);
    let v = vec![
        1,
        #[cfg(debug_assertions)]
        2,
        3,
    ];
    println!("{:?}", v);
    let t = (#[cfg(unix)] "u", 5);
    println!("{:?}", t);
}
