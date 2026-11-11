// Hand-written PartialEq / Eq, including cross-type comparisons.
struct Case(&'static str);

// case-insensitive ASCII compare
fn eq_ci(x: &str, y: &str) -> bool {
        let a = x.as_bytes();
        let b = y.as_bytes();
        if a.len() != b.len() {
            return false;
        }
        let mut i = 0;
        while i < a.len() {
            if a[i].to_ascii_lowercase() != b[i].to_ascii_lowercase() {
                return false;
            }
            i += 1;
        }
        true
}

impl PartialEq for Case {
    fn eq(&self, o: &Case) -> bool {
        eq_ci(self.0, o.0)
    }
}

impl PartialEq<&str> for Case {
    fn eq(&self, o: &&str) -> bool {
        eq_ci(self.0, o)
    }
}

#[derive(Clone, Copy)]
struct Frac {
    n: i32,
    d: i32,
}
impl PartialEq for Frac {
    fn eq(&self, o: &Frac) -> bool {
        self.n * o.d == o.n * self.d
    }
}
impl Eq for Frac {}
impl PartialEq<i32> for Frac {
    fn eq(&self, o: &i32) -> bool {
        self.n == *o * self.d
    }
}

fn count_matches<T: PartialEq>(xs: &[T], needle: &T) -> usize {
    let mut c = 0;
    for x in xs {
        if x == needle {
            c += 1;
        }
    }
    c
}

fn all_distinct<T: Eq>(xs: &[T]) -> bool {
    for i in 0..xs.len() {
        for j in i + 1..xs.len() {
            if xs[i] == xs[j] {
                return false;
            }
        }
    }
    true
}

fn main() {
    println!("{}", Case("Hello") == Case("hELLO"));
    println!("{}", Case("Hello") == Case("Help!"));
    println!("{}", Case("abc") != Case("ab"));
    println!("{}", Case("RUST") == "rust");
    let h = Frac { n: 1, d: 2 };
    let q = Frac { n: 2, d: 4 };
    let t = Frac { n: 1, d: 3 };
    println!("{} {} {}", h == q, h == t, h != t);
    println!("{} {}", Frac { n: 6, d: 3 } == 2, Frac { n: 5, d: 3 } == 1);
    let fs = [h, q, t, Frac { n: 3, d: 6 }];
    println!("{}", count_matches(&fs, &Frac { n: 10, d: 20 }));
    println!("{}", all_distinct(&fs));
    println!("{}", all_distinct(&[h, t]));
    println!("{}", count_matches(&[Case("a"), Case("A"), Case("b")], &Case("a")));
    // tuples and arrays of PartialEq types
    println!("{}", (1, 'a', true) == (1, 'a', true));
    println!("{}", [1, 2, 3] == [1, 2, 4]);
    println!("{}", (h, 5) == (q, 5));
}
