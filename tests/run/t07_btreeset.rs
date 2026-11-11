use std::collections::BTreeSet;
use std::ops::Bound::{Excluded, Included, Unbounded};

fn main() {
    let mut s: BTreeSet<i32> = [5, 3, 9, 1, 7, 3, 5, -2].into_iter().collect();
    println!("{:?} len={}", s, s.len());
    println!("first={:?} last={:?}", s.first(), s.last());
    println!("insert 4={} insert 9={}", s.insert(4), s.insert(9));
    println!("contains 7={} contains 8={}", s.contains(&7), s.contains(&8));
    println!("range 3..8={:?}", s.range(3..8).collect::<Vec<_>>());
    println!("range excl/incl={:?}", s.range((Excluded(3), Included(7))).collect::<Vec<_>>());
    println!("range unbounded/excl={:?}", s.range((Unbounded, Excluded(4))).collect::<Vec<_>>());
    println!("pop_first={:?} pop_last={:?} now {:?}", s.pop_first(), s.pop_last(), s);
    let t: BTreeSet<i32> = (2..6).collect();
    println!("t={:?}", t);
    println!("union={:?}", s.union(&t).collect::<Vec<_>>());
    println!("inter={:?}", s.intersection(&t).collect::<Vec<_>>());
    println!("diff={:?}", s.difference(&t).collect::<Vec<_>>());
    println!("sym={:?}", s.symmetric_difference(&t).collect::<Vec<_>>());
    println!("rev={:?}", s.iter().rev().collect::<Vec<_>>());
    let big = s.split_off(&5);
    println!("split low={:?} high={:?}", s, big);
    let mut words: BTreeSet<String> = BTreeSet::new();
    for w in "to be or not to be that is the question".split(' ') {
        words.insert(w.to_string());
    }
    println!("{:?}", words);
    words.retain(|w| w.len() > 2);
    println!("{:?}", words);
    let mut tuples = BTreeSet::new();
    for i in 0..6 {
        tuples.insert((i % 3, 10 - i));
    }
    println!("{:?}", tuples);
    println!("subset={}", t.is_subset(&(0..10).collect()));
    let mut it = tuples.iter();
    println!("next={:?} next_back={:?}", it.next(), it.next_back());
    println!("remove (1,9)={} remove (1,9)={}", tuples.remove(&(1, 9)), tuples.remove(&(1, 9)));
    println!("{:?}", tuples);
}
