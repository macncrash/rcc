// rcc-flags: --cfg feature="fast" --cfg my_flag
// #[cfg], #[cfg_attr] and cfg!() in every position, with features.
#![allow(dead_code, unused)]
#![cfg_attr(feature = "fast", allow(unused_mut))]

#[cfg(feature = "fast")]
use std::collections::HashMap as Map;
#[cfg(not(feature = "fast"))]
use std::collections::BTreeMap as Map;

#[cfg(all(unix, feature = "fast", not(feature = "slow")))]
fn mode() -> &'static str {
    "fast-unix"
}
#[cfg(not(all(unix, feature = "fast", not(feature = "slow"))))]
fn mode() -> &'static str {
    "other"
}

#[cfg(any(feature = "slow", my_flag))]
const FLAG: i32 = 1;
#[cfg(not(any(feature = "slow", my_flag)))]
const FLAG: i32 = 2;

#[cfg_attr(feature = "fast", derive(Debug, Clone))]
#[cfg_attr(not(feature = "fast"), derive(Debug))]
struct S {
    a: i32,
    #[cfg(feature = "slow")]
    b: String,
    #[cfg(feature = "fast")]
    c: u8,
}

#[derive(Debug)]
enum E {
    A,
    #[cfg(feature = "slow")]
    B,
    #[cfg_attr(feature = "fast", allow(unused))]
    C(#[cfg(feature = "fast")] i32, u8),
}

struct T;
impl T {
    #[cfg(feature = "fast")]
    fn get(&self) -> i32 {
        1
    }
    #[cfg(not(feature = "fast"))]
    fn get(&self) -> i32 {
        2
    }
    #[cfg(test)]
    fn only_test(&self) {}
}

trait Tr {
    #[cfg(feature = "fast")]
    fn f(&self) -> i32 {
        10
    }
    #[cfg(not(feature = "fast"))]
    fn f(&self) -> i32;
}
impl Tr for T {}

fn params(#[cfg(feature = "slow")] x: i32, y: i32) -> i32 {
    y
}

#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        panic!();
    }
}

#[test]
fn top_level_test() {
    unreachable!();
}

#[cfg(target_os = "nonexistent")]
mod missing_file_not_loaded;

mod inner {
    #![cfg(feature = "fast")]
    pub fn hello() -> &'static str {
        "inner hello"
    }
}

#[cfg(debug_assertions)]
fn dbg_only() {}

fn tail_cfg() -> i32 {
    #[cfg(feature = "fast")]
    {
        100
    }
    #[cfg(not(feature = "fast"))]
    {
        200
    }
}

fn tail_cfg2() -> &'static str {
    let x = 1;
    #[cfg(feature = "slow")]
    {
        "slow"
    }
    #[cfg(not(feature = "slow"))]
    {
        "not slow"
    }
}

fn main() {
    let mut m = Map::new();
    m.insert(1, "one");
    println!("{} {} {:?}", mode(), FLAG, m.get(&1));
    let s = S {
        a: 1,
        #[cfg(feature = "slow")]
        b: String::new(),
        #[cfg(feature = "fast")]
        c: 2,
    };
    println!("{:?} {:?}", s, s.clone());
    println!("{:?} {:?}", E::A, E::C(1, 2));
    println!("{} {} {}", T.get(), T.f(), params(5));
    println!("{}", inner::hello());
    println!("{} {}", tail_cfg(), tail_cfg2());
    #[cfg(feature = "fast")]
    let v = 1;
    #[cfg(not(feature = "fast"))]
    let v = 2;
    #[cfg(feature = "fast")]
    println!("stmt fast");
    #[cfg(not(feature = "fast"))]
    println!("stmt not fast");
    let arr = [1, #[cfg(feature = "slow")] 2, 3];
    let tup = (1, #[cfg(feature = "fast")] "f", 2);
    println!("{} {:?} {:?}", v, arr, tup);
    let k = match v {
        #[cfg(feature = "fast")]
        1 => "one-fast",
        #[cfg(not(feature = "fast"))]
        1 => "one-slow",
        _ => "other",
    };
    println!("{}", k);
    let vv = vec![1, #[cfg(feature = "slow")] 2, 3, #[cfg(my_flag)] 4];
    println!("{:?}", vv);
    println!(
        "{} {} {} {} {}",
        cfg!(unix),
        cfg!(feature = "fast"),
        cfg!(feature = "slow"),
        cfg!(all(unix, any(my_flag, test))),
        cfg!(test)
    );
    println!("{} {}", cfg!(debug_assertions), cfg!(target_pointer_width = "64"));
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        println!("known os");
    }
    #[cfg(unix)]
    {
        println!("unix block");
    }
}
