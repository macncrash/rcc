// impl Trait in argument and return position.
use std::fmt::Display;

trait Score {
    fn score(&self) -> u32;
}
struct Word(&'static str);
struct Num(u32);
impl Score for Word {
    fn score(&self) -> u32 {
        self.0.len() as u32
    }
}
impl Score for Num {
    fn score(&self) -> u32 {
        self.0 % 10
    }
}

fn total(a: &impl Score, b: &impl Score) -> u32 {
    a.score() + b.score()
}

fn show(x: impl Display) -> String {
    format!("[{}]", x)
}

fn make_scorer(word: bool) -> impl Score {
    struct Fixed(u32);
    impl Score for Fixed {
        fn score(&self) -> u32 {
            self.0
        }
    }
    if word { Fixed(100) } else { Fixed(1) }
}

fn evens(limit: u32) -> impl Iterator<Item = u32> {
    (0..limit).filter(|x| x % 2 == 0)
}

fn adder(k: i32) -> impl Fn(i32) -> i32 {
    move |x| x + k
}

fn sum_iter(it: impl Iterator<Item = u32>) -> u32 {
    let mut s = 0;
    for x in it {
        s += x;
    }
    s
}

fn main() {
    println!("{}", total(&Word("hello"), &Num(47)));
    println!("{}", total(&Num(3), &Num(99)));
    println!("{}", show(5));
    println!("{}", show("abc"));
    println!("{}", show(1.25));
    println!("{}", make_scorer(true).score());
    println!("{}", make_scorer(false).score());
    for e in evens(9) {
        print!("{} ", e);
    }
    println!();
    println!("{}", sum_iter(evens(101)));
    println!("{}", sum_iter(1..=4));
    let add5 = adder(5);
    let sub2 = adder(-2);
    println!("{} {} {}", add5(10), sub2(10), add5(sub2(0)));
}
