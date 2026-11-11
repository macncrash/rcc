use std::collections::HashMap;
use std::collections::hash_map::Entry;

#[derive(Debug, Default, Clone)]
struct Stats {
    count: u32,
    total: i64,
}

fn main() {
    let text = "the quick brown fox jumps over the lazy dog the fox";
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for w in text.split_whitespace() {
        *counts.entry(w).or_insert(0) += 1;
    }
    let mut cv: Vec<_> = counts.iter().collect();
    cv.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (w, c) in &cv {
        println!("{} {}", w, c);
    }

    let mut stats: HashMap<char, Stats> = HashMap::new();
    for (i, ch) in "abracadabra".chars().enumerate() {
        let s = stats.entry(ch).or_default();
        s.count += 1;
        s.total += i as i64;
    }
    let mut sk: Vec<_> = stats.keys().copied().collect();
    sk.sort();
    for k in sk {
        println!("{:?} -> {:?}", k, stats[&k]);
    }

    let mut calls = 0;
    let mut lazy: HashMap<i32, String> = HashMap::new();
    for k in [3, 1, 3, 2, 1, 3] {
        let v = lazy.entry(k).or_insert_with(|| {
            calls += 1;
            format!("val{}", k * 11)
        });
        v.push('!');
    }
    println!("or_insert_with calls={}", calls);
    let mut lk: Vec<_> = lazy.iter().collect();
    lk.sort();
    println!("{:?}", lk);

    let mut m: HashMap<String, Vec<u32>> = HashMap::new();
    for (k, v) in [("a", 1), ("b", 2), ("a", 3), ("c", 4), ("a", 5), ("b", 6)] {
        m.entry(k.to_string()).or_insert_with(Vec::new).push(v);
    }
    m.entry("a".to_string()).and_modify(|v| v.reverse()).or_insert_with(Vec::new);
    m.entry("z".to_string()).and_modify(|v| v.push(100)).or_insert(vec![42]);
    let mut mk: Vec<_> = m.iter().collect();
    mk.sort();
    println!("{:?}", mk);

    match m.entry("b".to_string()) {
        Entry::Occupied(mut o) => {
            println!("occupied key={} val={:?}", o.key(), o.get());
            o.get_mut().push(7);
            println!("now {:?}", o.get());
        }
        Entry::Vacant(_) => println!("vacant?"),
    }
    match m.entry("q".to_string()) {
        Entry::Occupied(_) => println!("occupied?"),
        Entry::Vacant(v) => {
            println!("vacant key={}", v.key());
            v.insert(vec![9, 9]);
        }
    }
    if let Entry::Occupied(o) = m.entry("c".to_string()) {
        let (k, v) = o.remove_entry();
        println!("removed {} {:?}", k, v);
    }
    let mut mk: Vec<_> = m.into_iter().collect();
    mk.sort();
    println!("{:?}", mk);
    let k = *counts.entry("dog").key();
    println!("entry key {}", k);
}
