// derive(PartialEq, Eq, PartialOrd, Ord): lexicographic field order,
// enum variant order, and use in sort / max / comparisons.
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    prio: u8,
    seq: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Low,
    Mid(i32),
    High { level: u8, name: String },
}

#[derive(Debug, PartialEq, PartialOrd)]
struct F(f64, u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Wrap(Key, bool);

fn main() {
    let a = Key { prio: 1, seq: 5 };
    let b = Key { prio: 1, seq: -3 };
    let c = Key { prio: 0, seq: 100 };
    println!("{} {} {} {}", a == b, a > b, b > c, a.cmp(&c) == Ordering::Greater);
    let mut ks = vec![a, b, c, Key { prio: 2, seq: 0 }, Key { prio: 0, seq: -1 }];
    ks.sort();
    println!("{:?}", ks);
    println!("{:?}", ks.iter().max());
    ks.sort_by(|x, y| y.cmp(x));
    println!("{:?}", ks[0]);

    let mut kinds = vec![
        Kind::High { level: 1, name: String::from("b") },
        Kind::Mid(3),
        Kind::Low,
        Kind::High { level: 1, name: String::from("a") },
        Kind::Mid(-7),
        Kind::High { level: 0, name: String::from("z") },
    ];
    kinds.sort();
    for k in &kinds {
        println!("{:?}", k);
    }
    println!("{}", Kind::Low < Kind::Mid(i32::MIN));
    println!("{}", Kind::Mid(5) == Kind::Mid(5));
    println!("{}", Kind::Mid(5) != Kind::Mid(6));

    let x = F(1.0, 2);
    let y = F(f64::NAN, 0);
    println!("{:?}", x.partial_cmp(&F(1.0, 1)));
    println!("{:?}", x.partial_cmp(&y));
    println!("{} {}", x < y, x == F(1.0, 2));

    let w1 = Wrap(a, false);
    let w2 = Wrap(a, true);
    println!("{} {:?}", w1 < w2, w1.max(w2));
    println!("{:?}", (1, 'b', "x").cmp(&(1, 'a', "z")));
    println!("{:?}", [1, 2, 3][..].cmp(&[1, 2][..]));
}
