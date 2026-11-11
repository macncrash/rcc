use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};

fn main() {
    let words = ["pear", "fig", "apple", "fig", "kiwi", "pear", "date"];
    let set: BTreeSet<&str> = words.iter().copied().collect();
    println!("{:?}", set);
    let hs: HashSet<&str> = words.iter().copied().collect();
    println!("hs len={}", hs.len());
    let lens: BTreeMap<&str, usize> = words.iter().map(|w| (*w, w.len())).collect();
    println!("{:?}", lens);
    let hm: HashMap<usize, &str> = words.iter().enumerate().map(|(i, w)| (i, *w)).collect();
    let mut hk: Vec<_> = hm.iter().filter(|(_, w)| w.len() == 4).map(|(i, _)| *i).collect();
    hk.sort();
    println!("{:?}", hk);
    let dq: VecDeque<usize> = words.iter().map(|w| w.len()).collect();
    println!("{:?}", dq);
    let heap: BinaryHeap<&str> = words.iter().copied().collect();
    println!("{:?}", heap.into_sorted_vec());
    let from_arr = BTreeMap::from([(3, 'c'), (1, 'a'), (2, 'b')]);
    println!("{:?}", from_arr);
    let hm2 = HashMap::from([("k1", 1), ("k2", 2)]);
    let mut v: Vec<_> = hm2.into_iter().collect();
    v.sort();
    println!("{:?}", v);
    let bs = BTreeSet::from([5, 1, 3]);
    println!("{:?}", bs);
    let mut m: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for w in words {
        m.entry(w[..1].to_string()).or_default().push(w);
    }
    println!("{:?}", m);
    let sets: Vec<BTreeSet<char>> = words.iter().map(|w| w.chars().collect()).collect();
    let all: BTreeSet<char> = sets.iter().flatten().copied().collect();
    println!("{:?}", all);
    let res: Result<BTreeMap<&str, i32>, String> = [("a", "1"), ("b", "2")]
        .iter()
        .map(|(k, v)| v.parse::<i32>().map(|n| (*k, n)).map_err(|e| e.to_string()))
        .collect();
    println!("{:?}", res);
    let bad: Result<BTreeSet<i32>, std::num::ParseIntError> = ["1", "x"].iter().map(|s| s.parse()).collect();
    println!("{}", bad.is_err());
    let unzipped: (BTreeSet<i32>, Vec<char>) = [(2, 'x'), (1, 'y'), (2, 'z')].into_iter().unzip();
    println!("{:?}", unzipped);
}
