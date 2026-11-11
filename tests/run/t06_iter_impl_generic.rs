struct Pairs<'a, T> {
    s: &'a [T],
    i: usize,
}
impl<'a, T> Iterator for Pairs<'a, T> {
    type Item = (&'a T, &'a T);
    fn next(&mut self) -> Option<Self::Item> {
        if self.i + 1 < self.s.len() {
            let r = (&self.s[self.i], &self.s[self.i + 1]);
            self.i += 2;
            Some(r)
        } else {
            None
        }
    }
}
struct Countdown(i32);
impl Iterator for Countdown {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        if self.0 <= 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0 + 1)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.0.max(0) as usize;
        (n, Some(n))
    }
}
impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<i32> {
        None
    }
}
struct Grid {
    w: usize,
    h: usize,
}
impl Grid {
    fn cells(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        (0..self.h).flat_map(move |y| (0..self.w).map(move |x| (x, y)))
    }
}
fn sum_all<I: Iterator<Item = i32>>(it: I) -> i32 {
    let mut s = 0;
    for x in it {
        s += x;
    }
    s
}
fn main() {
    let data = [1, 2, 3, 4, 5];
    let p = Pairs { s: &data, i: 0 };
    for (a, b) in p {
        println!("{} {}", a, b);
    }
    let words = ["a", "b", "c", "d"];
    let joined: Vec<String> = Pairs { s: &words, i: 0 }.map(|(x, y)| format!("{}{}", x, y)).collect();
    println!("{:?}", joined);
    let cd = Countdown(4);
    println!("hint {:?}", cd.size_hint());
    println!("{:?}", cd.collect::<Vec<_>>());
    println!("sum {}", sum_all(Countdown(10)));
    println!("sum {}", sum_all(vec![5, 6].into_iter()));
    let g = Grid { w: 3, h: 2 };
    println!("{:?}", g.cells().collect::<Vec<_>>());
    println!("{}", g.cells().filter(|(x, y)| x == y).count());
    let mut cd2 = Countdown(3);
    let by_ref: Vec<i32> = cd2.by_ref().take(1).collect();
    println!("{:?} rest {:?}", by_ref, cd2.collect::<Vec<_>>());
}
