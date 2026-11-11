enum List {
    Cons(i32, Box<List>),
    Nil,
}

use List::{Cons, Nil};

fn sum(l: &List) -> i32 {
    match l {
        Cons(v, rest) => v + sum(rest),
        Nil => 0,
    }
}

fn len(l: &List) -> usize {
    let mut n = 0;
    let mut cur = l;
    while let Cons(_, next) = cur {
        n += 1;
        cur = next;
    }
    n
}

fn print_list(l: &List) {
    let mut cur = l;
    loop {
        match cur {
            Cons(v, next) => {
                print!("{} -> ", v);
                cur = next;
            }
            Nil => {
                println!("nil");
                break;
            }
        }
    }
}

fn push_front(l: List, v: i32) -> List {
    Cons(v, Box::new(l))
}

fn reverse(l: List) -> List {
    let mut out = Nil;
    let mut cur = l;
    while let Cons(v, next) = cur {
        out = Cons(v, Box::new(out));
        cur = *next;
    }
    out
}

fn add_all(l: &mut List, d: i32) {
    let mut cur = l;
    while let Cons(v, next) = cur {
        *v += d;
        cur = &mut **next;
    }
}

fn main() {
    let mut l = Nil;
    for i in 1..=6 {
        l = push_front(l, i * i);
    }
    print_list(&l);
    println!("sum {} len {}", sum(&l), len(&l));
    let mut r = reverse(l);
    print_list(&r);
    add_all(&mut r, 100);
    print_list(&r);
    println!("sum {}", sum(&r));
    let b = Box::new(41);
    let v = *b + 1;
    println!("box {}", v);
    let mut bm = Box::new((1, 2));
    bm.0 += 10;
    (*bm).1 *= 5;
    println!("{} {}", bm.0, bm.1);
}
