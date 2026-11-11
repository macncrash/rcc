// Deref / DerefMut on wrappers: auto-deref for method calls, deref coercion.
use std::ops::{Deref, DerefMut};

struct Guarded<T> {
    inner: T,
    reads: std::cell::Cell<u32>,
}

impl<T> Guarded<T> {
    fn new(inner: T) -> Self {
        Guarded { inner, reads: std::cell::Cell::new(0) }
    }
}

impl<T> Deref for Guarded<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.reads.set(self.reads.get() + 1);
        &self.inner
    }
}
impl<T> DerefMut for Guarded<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

struct Name(String);
impl Deref for Name {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

fn takes_str(s: &str) -> usize {
    s.len()
}
fn takes_slice(s: &[i32]) -> i32 {
    let mut t = 0;
    for x in s {
        t += x;
    }
    t
}

fn main() {
    let mut g = Guarded::new(vec![1, 2, 3]);
    println!("{}", g.len());
    g.push(4);
    println!("{}", g[3]);
    println!("{}", takes_slice(&g));
    (*g).push(10);
    println!("{:?}", *g);
    println!("reads {}", g.reads.get());
    let n = Name(String::from("deref"));
    println!("{}", n.len());
    println!("{}", n.to_uppercase());
    println!("{}", takes_str(&n));
    println!("{}", n.starts_with("de"));
    let b = Box::new(Name(String::from("boxed")));
    println!("{}", takes_str(&b));
    let r = &&&n;
    println!("{}", r.len());
    let mut gs = Guarded::new(String::from("x"));
    gs.push_str("yz");
    println!("{} {}", *gs, gs.len());
    println!("reads {}", gs.reads.get());
}
