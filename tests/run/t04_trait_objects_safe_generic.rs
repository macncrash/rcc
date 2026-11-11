// Traits with generic params and associated types used together; iterator-like
// trait implemented by hand (no std Iterator) and adapters as generic structs.
trait Gen {
    type Out;
    fn next_val(&mut self) -> Option<Self::Out>;
}

struct Count {
    n: u32,
    max: u32,
}
impl Gen for Count {
    type Out = u32;
    fn next_val(&mut self) -> Option<u32> {
        if self.n < self.max {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }
}

struct MapG<G, F> {
    g: G,
    f: F,
}
impl<G: Gen, R, F: FnMut(G::Out) -> R> Gen for MapG<G, F> {
    type Out = R;
    fn next_val(&mut self) -> Option<R> {
        match self.g.next_val() {
            Some(v) => Some((self.f)(v)),
            None => None,
        }
    }
}

struct FilterG<G, P> {
    g: G,
    p: P,
}
impl<G: Gen, P: Fn(&G::Out) -> bool> Gen for FilterG<G, P> {
    type Out = G::Out;
    fn next_val(&mut self) -> Option<G::Out> {
        loop {
            let v = self.g.next_val()?;
            if (self.p)(&v) {
                return Some(v);
            }
        }
    }
}

fn collect_all<G: Gen>(mut g: G) -> Vec<G::Out> {
    let mut v = Vec::new();
    while let Some(x) = g.next_val() {
        v.push(x);
    }
    v
}

fn main() {
    println!("{:?}", collect_all(Count { n: 0, max: 5 }));
    let sq = MapG { g: Count { n: 0, max: 6 }, f: |x: u32| x * x };
    println!("{:?}", collect_all(sq));
    let odd = FilterG { g: Count { n: 0, max: 10 }, p: |x: &u32| x % 2 == 1 };
    println!("{:?}", collect_all(odd));
    let mut calls = 0;
    let chain = MapG {
        g: FilterG { g: Count { n: 10, max: 20 }, p: |x: &u32| x % 3 == 0 },
        f: |x: u32| {
            calls += 1;
            (x, x as f64 / 2.0)
        },
    };
    println!("{:?}", collect_all(chain));
    println!("calls {}", calls);
    let labels = MapG { g: Count { n: 0, max: 3 }, f: |x: u32| format!("#{}", x) };
    println!("{:?}", collect_all(labels));
}
