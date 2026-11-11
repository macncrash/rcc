use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

#[derive(Debug, PartialEq, Eq)]
struct Event {
    time: u64,
    seq: u32,
    kind: &'static str,
}

impl Ord for Event {
    fn cmp(&self, o: &Self) -> Ordering {
        // min-heap on (time, seq)
        o.time.cmp(&self.time).then_with(|| o.seq.cmp(&self.seq))
    }
}
impl PartialOrd for Event {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

fn top_k(xs: &[i64], k: usize) -> Vec<i64> {
    let mut h: BinaryHeap<Reverse<i64>> = BinaryHeap::new();
    for &x in xs {
        h.push(Reverse(x));
        if h.len() > k {
            h.pop();
        }
    }
    let mut v: Vec<i64> = h.into_iter().map(|Reverse(x)| x).collect();
    v.sort_by(|a, b| b.cmp(a));
    v
}

fn main() {
    let mut q = BinaryHeap::new();
    let mut seq = 0;
    let mut push = |q: &mut BinaryHeap<Event>, t: u64, k: &'static str| {
        seq += 1;
        q.push(Event { time: t, seq, kind: k });
    };
    push(&mut q, 5, "arrive");
    push(&mut q, 1, "arrive");
    push(&mut q, 5, "tick");
    push(&mut q, 3, "arrive");
    let mut processed = 0;
    while let Some(ev) = q.pop() {
        println!("t={} seq={} {}", ev.time, ev.seq, ev.kind);
        processed += 1;
        if ev.kind == "arrive" && ev.time < 10 {
            push(&mut q, ev.time + 4, "depart");
        }
        if processed > 20 {
            break;
        }
    }
    let xs: Vec<i64> = (0..40).map(|i: i64| (i * 37 + 11) % 101 - 50).collect();
    println!("top5={:?}", top_k(&xs, 5));
    println!("top1={:?} top0={:?}", top_k(&xs, 1), top_k(&xs, 0));
    let mut h: BinaryHeap<(u8, Reverse<&str>)> = BinaryHeap::new();
    for (p, n) in [(1, "b"), (3, "z"), (3, "a"), (2, "q"), (1, "a")] {
        h.push((p, Reverse(n)));
    }
    let order: Vec<String> = std::iter::from_fn(|| h.pop()).map(|(p, Reverse(n))| format!("{}{}", p, n)).collect();
    println!("{}", order.join(","));
}
