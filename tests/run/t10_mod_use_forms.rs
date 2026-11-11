// `use` with globs, renames, self, super, crate, nested groups, in fn bodies.
mod a {
    pub mod b {
        pub fn one() -> i32 { 1 }
        pub fn two() -> i32 { 2 }
        pub const K: i32 = 30;
        pub struct S(pub i32);
        pub mod c {
            use super::super::alpha as al;
            pub fn three() -> i32 { 3 + al() }
            pub fn via_super() -> i32 { super::two() * 100 }
        }
    }
    pub fn alpha() -> i32 { 0 }
    pub enum Color { Red, Green, Blue }
}

use a::b::{self, c::{self as cc, three}, one as uno, K};
use crate::a::Color::*;
use std::collections::{BTreeMap as Map, BTreeSet};
use std::fmt::Write as _;

fn color_name(c: a::Color) -> &'static str {
    match c {
        Red => "red",
        Green => "green",
        Blue => "blue",
    }
}

mod user {
    use super::a::b::*;
    pub fn sum() -> i32 {
        one() + two() + K + S(5).0
    }
}

fn main() {
    println!("{} {} {} {}", uno(), b::two(), three(), K);
    println!("{} {}", cc::via_super(), cc::three());
    println!("{} {} {}", color_name(Red), color_name(Green), color_name(Blue));
    println!("user sum {}", user::sum());
    let mut m = Map::new();
    m.insert(2, "two");
    m.insert(1, "one");
    for (k, v) in &m {
        println!("{} -> {}", k, v);
    }
    let mut s = BTreeSet::new();
    s.insert(9);
    s.insert(3);
    println!("{:?}", s);
    {
        use a::b::two as deux;
        use std::cmp::{max, min};
        println!("deux {} max {} min {}", deux(), max(4, 9), min(4, 9));
    }
    let mut out = String::new();
    write!(out, "{}-{}", 1, 2).unwrap();
    println!("{}", out);
}
