enum Slot {
    Empty,
    Full(u32),
    Pair(u32, u32),
}

fn val(s: &Slot) -> u32 {
    if let Slot::Full(n) = s {
        *n
    } else if let Slot::Pair(a, b) = s {
        a + b
    } else {
        0
    }
}

fn main() {
    let slots = [Slot::Empty, Slot::Full(7), Slot::Pair(2, 3), Slot::Full(100)];
    let mut total = 0;
    for i in 0..slots.len() {
        let v = val(&slots[i]);
        total += v;
        println!("slot {} val {}", i, v);
        if let Slot::Full(n) = slots[i] {
            println!("  full {}", n);
        }
        if let Slot::Full(n @ 50..=200) = slots[i] {
            println!("  big full {}", n);
        }
        if let Slot::Empty = slots[i] {
            println!("  empty");
        }
    }
    println!("total {}", total);
    let pair = (1, Slot::Pair(9, 9));
    if let (1, Slot::Pair(x, y)) = pair {
        println!("nested {} {}", x, y);
    }
    let mut s = Slot::Full(1);
    if let Slot::Full(ref mut n) = s {
        *n += 41;
    }
    println!("mutated {}", val(&s));
    let x = 5;
    let r = if let 0..=9 = x { "digit" } else { "nope" };
    println!("{}", r);
    let t = (3, 4);
    if let (a, 4) = t {
        println!("a {}", a);
    }
    let res = if let Slot::Pair(a, _) = s { a } else { 999 };
    println!("res {}", res);
}
