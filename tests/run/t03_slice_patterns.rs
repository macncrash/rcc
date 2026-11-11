fn describe(xs: &[i32]) -> i32 {
    match xs {
        [] => {
            println!("empty");
            0
        }
        [x] => {
            println!("one {}", x);
            *x
        }
        [a, b] => {
            println!("two {} {}", a, b);
            a + b
        }
        [first, .., last] if first == last => {
            println!("same ends {}", first);
            *first
        }
        [first, middle @ .., last] => {
            println!("first {} last {} middle len {}", first, last, middle.len());
            first * last + middle.len() as i32
        }
    }
}

fn sum(xs: &[i32]) -> i32 {
    match xs {
        [] => 0,
        [h, rest @ ..] => h + sum(rest),
    }
}

fn starts(xs: &[u8]) -> &'static str {
    match xs {
        [0x7f, b'E', b'L', b'F', ..] => "elf",
        [b'#', b'!', ..] => "script",
        [.., 0] => "nul-terminated",
        _ => "unknown",
    }
}

fn main() {
    let total = describe(&[]) + describe(&[5]) + describe(&[2, 3]) + describe(&[4, 1, 4])
        + describe(&[1, 2, 3, 4, 5, 6]);
    println!("total {}", total);
    println!("sum {}", sum(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
    println!("{}", starts(&[0x7f, b'E', b'L', b'F', 2, 1]));
    println!("{}", starts(b"#!/bin/sh"));
    println!("{}", starts(&[1, 2, 0]));
    println!("{}", starts(b"xy"));
    let arr = [10, 20, 30, 40];
    let [a, b, c, d] = arr;
    println!("{} {} {} {}", a, b, c, d);
    let [x, .., y] = arr;
    println!("{} {}", x, y);
    let [_, rest @ ..] = arr;
    println!("rest {} {}", rest.len(), rest[0]);
    if let [p, q, ..] = &arr[1..] {
        println!("p {} q {}", p, q);
    }
    let mut m = [1, 2, 3];
    if let [first, .., last] = &mut m[..] {
        let t = *first;
        *first = *last;
        *last = t;
    }
    println!("{} {} {}", m[0], m[1], m[2]);
    let words = ["a", "bb", "ccc"];
    match &words[..] {
        [w, ..] => println!("first word {}", w),
        [] => println!("none"),
    }
}
