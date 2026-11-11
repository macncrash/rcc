// Hand-written PartialOrd and Ord; comparison operators, cmp, max/min, sort.
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl Ord for Version {
    fn cmp(&self, o: &Self) -> Ordering {
        match self.major.cmp(&o.major) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.minor.cmp(&o.minor) {
            Ordering::Equal => self.patch.cmp(&o.patch),
            ord => ord,
        }
    }
}
impl PartialOrd for Version {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

// Reverse ordering by length, only partially ordered when NaN-like.
#[derive(Debug, PartialEq)]
struct Score(f64);
impl PartialOrd for Score {
    fn partial_cmp(&self, o: &Score) -> Option<Ordering> {
        o.0.partial_cmp(&self.0)
    }
}

fn v(a: u32, b: u32, c: u32) -> Version {
    Version { major: a, minor: b, patch: c }
}

fn ord_name(o: Ordering) -> &'static str {
    match o {
        Ordering::Less => "Less",
        Ordering::Equal => "Equal",
        Ordering::Greater => "Greater",
    }
}

fn main() {
    let a = v(1, 2, 3);
    let b = v(1, 10, 0);
    let c = v(0, 99, 99);
    println!("{} {} {} {}", a < b, a > c, b >= b, c <= a);
    println!("{}", ord_name(a.cmp(&b)));
    println!("{}", ord_name(b.cmp(&a)));
    println!("{}", ord_name(a.cmp(&a)));
    println!("{:?}", a.max(c));
    println!("{:?}", b.min(c));
    println!("{:?}", v(2, 0, 0).clamp(a, b));
    let mut vs = vec![b, c, v(1, 2, 0), a, v(1, 2, 10)];
    vs.sort();
    for x in &vs {
        println!("{}.{}.{}", x.major, x.minor, x.patch);
    }
    let s1 = Score(1.0);
    let s2 = Score(2.0);
    let n = Score(f64::NAN);
    println!("{} {} {}", s1 < s2, s1 > s2, s1 == s2);
    println!("{:?}", s1.partial_cmp(&s2));
    println!("{:?}", s1.partial_cmp(&n));
    println!("{} {}", s1 < n, s1 >= n);
    println!("{}", ord_name(a.cmp(&b).reverse()));
    println!("{}", ord_name(1.cmp(&1).then(3.cmp(&2))));
    println!("{}", ord_name(a.cmp(&a).then_with(|| 5.cmp(&9))));
}
