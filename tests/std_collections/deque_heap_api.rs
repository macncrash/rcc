// VecDeque (wraparound, insert/remove, drain, rotate, contiguity) and
// BinaryHeap (internal order after every operation matches std).
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

fn main() {
    let mut d: VecDeque<u32> = VecDeque::with_capacity(4);
    for i in 0..4 { d.push_back(i); }
    d.pop_front();
    d.pop_front();
    d.push_back(4);
    d.push_back(5); // wraps
    d.push_front(100);
    println!("{:?} len={} front={:?} back={:?} d[1]={}", d, d.len(), d.front(), d.back(), d[1]);
    let (a, b) = d.as_slices();
    println!("slices total={} contains 5={} contains 6={}", a.len() + b.len(), d.contains(&5), d.contains(&6));
    *d.front_mut().unwrap() += 1;
    *d.back_mut().unwrap() += 1;
    d[2] = 33;
    if let Some(x) = d.get_mut(3) { *x = 44; }
    println!("{:?} get(9)={:?}", d, d.get(9));
    d.insert(1, 7);
    d.insert(d.len(), 8);
    d.insert(0, 9);
    println!("insert {:?}", d);
    println!("remove(2)={:?} remove(99)={:?} {:?}", d.remove(2), d.remove(99), d);
    d.rotate_left(2);
    println!("rotl {:?}", d);
    d.rotate_right(3);
    println!("rotr {:?}", d);
    d.swap(0, 1);
    println!("swap {:?}", d);
    let dr: Vec<u32> = d.drain(1..3).collect();
    println!("drain {:?} left {:?}", dr, d);
    let dr2: Vec<u32> = d.drain(d.len() - 2..).collect();
    println!("drain tail {:?} left {:?}", dr2, d);
    d.extend([1, 2, 3, 4, 5, 6, 7, 8]);
    d.retain(|x| x % 2 == 0);
    println!("retain {:?}", d);
    d.retain_mut(|x| { *x += 1; *x < 8 });
    println!("retain_mut {:?}", d);
    d.make_contiguous().sort();
    println!("sorted {:?} contiguous_len={}", d, d.as_slices().0.len());
    let tail = d.split_off(2);
    println!("split {:?} {:?}", d, tail);
    let mut t2 = tail.clone();
    d.append(&mut t2);
    println!("append {:?} other empty={}", d, t2.is_empty());
    d.truncate(3);
    d.resize(5, 0);
    println!("truncate+resize {:?}", d);
    println!("iter rev {:?} sum={}", d.iter().rev().collect::<Vec<_>>(), d.iter().sum::<u32>());
    for x in d.iter_mut() { *x *= 10; }
    for x in &mut d { *x += 1; }
    println!("iter_mut {:?}", d);
    let v: Vec<u32> = d.clone().into_iter().collect();
    let back: Vec<u32> = Vec::from(d.clone());
    println!("into_iter {:?} into vec {:?} eq={}", v, back, v == back);
    let from_vec: VecDeque<i8> = vec![1, 2, 3].into();
    let from_arr = VecDeque::from([1i8, 2, 3]);
    println!("eq {} {} {}", from_vec == from_arr, from_vec == vec![1, 2, 3], from_vec == [1, 2, 3]);
    println!("pops {:?} {:?} {:?} {:?}", d.pop_back(), d.pop_front(), d.swap_remove_back(0), d.swap_remove_front(1));
    println!("{:?} {:#?}", d, VecDeque::from(vec!["x"]));
    d.clear();
    let pf = d.pop_front();
    println!("cleared {:?} {:?} {:?}", d, pf, d.front());
    let mut q: VecDeque<usize> = VecDeque::new();
    let mut total = 0usize;
    for i in 0..100_000 {
        q.push_back(i);
        if i % 3 == 0 { total += q.pop_front().unwrap(); }
        if i % 7 == 0 { q.push_front(i); }
    }
    println!("queue len={} total={} sum={}", q.len(), total, q.iter().sum::<usize>());

    // BinaryHeap internal layout must match std
    let mut h = BinaryHeap::new();
    for x in [5, 1, 8, 3, 9, 2, 7, 4, 6, 0] {
        h.push(x);
        println!("push {} -> {:?}", x, h);
    }
    println!("peek={:?} len={}", h.peek(), h.len());
    while h.len() > 5 {
        let top = h.pop();
        println!("pop {:?} -> {:?}", top, h);
    }
    h.extend(vec![11, 12, 13]);
    println!("extend -> {:?}", h);
    h.extend(vec![-1]);
    println!("extend small -> {:?}", h);
    if let Some(mut top) = h.peek_mut() { *top = -5; }
    println!("peek_mut -> {:?}", h);
    if let Some(top) = h.peek_mut() { println!("peek_mut read {}", *top); }
    println!("unmodified -> {:?}", h);
    h.retain(|x| x % 2 != 0);
    println!("retain -> {:?}", h);
    let mut other = BinaryHeap::from(vec![20, 21, 22, 23, 24, 25]);
    println!("from vec -> {:?}", other);
    h.append(&mut other);
    println!("append -> {:?} other={:?}", h, other);
    println!("into_sorted_vec={:?}", h.clone().into_sorted_vec());
    println!("into_vec={:?}", h.clone().into_vec());
    let mut mh: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();
    for (i, w) in [7u32, 3, 9, 3, 1].iter().enumerate() { mh.push(Reverse((*w, i))); }
    print!("min-heap order:");
    while let Some(Reverse((w, i))) = mh.pop() { print!(" ({},{})", w, i); }
    println!();
    let col: BinaryHeap<&str> = ["pear", "apple", "fig", "kiwi"].into_iter().collect();
    println!("collect -> {:?} sum_len={}", col, col.iter().map(|s| s.len()).sum::<usize>());
    let mut dh = col.clone();
    let mut drained: Vec<&str> = dh.drain().collect();
    drained.sort();
    println!("drain {:?} empty={}", drained, dh.is_empty());
    let big: BinaryHeap<u64> = (0..1000u64).map(|i| i.wrapping_mul(2654435761) % 1009).collect();
    let sorted = big.into_sorted_vec();
    println!("big sorted ok={} first={} last={}", sorted.windows(2).all(|w| w[0] <= w[1]), sorted[0], sorted[999]);
}
