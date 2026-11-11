struct Counter {
    n: u32,
    limit: u32,
}
impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.n < self.limit {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }
}
struct Fib {
    a: u64,
    b: u64,
}
impl Iterator for Fib {
    type Item = u64;
    fn next(&mut self) -> Option<u64> {
        let r = self.a;
        self.a = self.b;
        self.b += r;
        Some(r)
    }
}
fn main() {
    let c = Counter { n: 0, limit: 5 };
    let v: Vec<u32> = c.collect();
    println!("{:?}", v);
    let s: u32 = Counter { n: 0, limit: 5 }
        .zip(Counter { n: 0, limit: 5 }.skip(1))
        .map(|(a, b)| a * b)
        .filter(|x| x % 3 == 0)
        .sum();
    println!("{}", s);
    for x in (Counter { n: 10, limit: 13 }) {
        println!("for {}", x);
    }
    let f: Vec<u64> = Fib { a: 0, b: 1 }.take_while(|&x| x < 100).collect();
    println!("{:?}", f);
    let big = Fib { a: 0, b: 1 }.nth(80).unwrap();
    println!("{}", big);
    let mut it = Counter { n: 0, limit: 3 };
    println!("{:?} {:?} {:?} {:?} {:?}", it.next(), it.next(), it.next(), it.next(), it.next());
    let evens = Fib { a: 0, b: 1 }.filter(|x| x % 2 == 0).take(5).collect::<Vec<_>>();
    println!("{:?}", evens);
    let mx = Counter { n: 0, limit: 7 }.max_by_key(|x| (x * 5) % 7);
    println!("{:?}", mx);
    let cnt = Counter { n: 0, limit: 100 }.step_by(7).count();
    println!("{}", cnt);
}
