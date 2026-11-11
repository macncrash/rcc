// BTreeMap / BTreeSet API: ordered iteration, ranges, entry, first/last,
// split_off/append, and a large randomized check against a sorted model.
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Included, Unbounded};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn main() {
    let mut m: BTreeMap<i32, String> = BTreeMap::new();
    for k in [50, 10, 40, 20, 30, 60, 0] {
        m.insert(k, format!("v{}", k));
    }
    println!("{:?}", m);
    println!("len={} first={:?} last={:?}", m.len(), m.first_key_value(), m.last_key_value());
    println!("get 20={:?} get 25={:?} m[&40]={}", m.get(&20), m.get(&25), m[&40]);
    println!("insert existing -> {:?}", m.insert(20, "twenty".into()));
    println!("keys={:?}", m.keys().collect::<Vec<_>>());
    println!("values rev={:?}", m.values().rev().collect::<Vec<_>>());
    println!("range 15..45 = {:?}", m.range(15..45).collect::<Vec<_>>());
    println!("range ..=30 = {:?}", m.range(..=30).map(|(k, _)| *k).collect::<Vec<_>>());
    println!("range 30.. = {:?}", m.range(30..).map(|(k, _)| *k).collect::<Vec<_>>());
    println!("range ..30 = {:?}", m.range(..30).map(|(k, _)| *k).collect::<Vec<_>>());
    println!("range 10..=50 rev = {:?}", m.range(10..=50).rev().map(|(k, _)| *k).collect::<Vec<_>>());
    println!("range .. = {}", m.range(..).count());
    println!("range (Excluded 10, Excluded 50) = {:?}", m.range((Excluded(10), Excluded(50))).map(|(k, _)| *k).collect::<Vec<_>>());
    println!("range (Excluded 30, Included 30) = {:?}", m.range((Excluded(30), Included(30))).count());
    println!("range (Unbounded, Included 0) = {:?}", m.range((Unbounded, Included(0))).map(|(k, _)| *k).collect::<Vec<_>>());
    println!("range 1000.. = {:?}", m.range(1000..).next());
    let mut r = m.range(0..100);
    println!("mixed ends: {:?} {:?} {:?}", r.next().map(|x| *x.0), r.next_back().map(|x| *x.0), r.next().map(|x| *x.0));
    for (_, v) in m.range_mut(20..=40) { v.push('!'); }
    println!("after range_mut: {:?}", m);
    for v in m.values_mut() { v.insert(0, '<'); }
    for (k, v) in m.iter_mut() { if *k == 0 { v.clear(); } }
    println!("after values_mut/iter_mut: {:?}", m);
    println!("remove 40={:?} remove 40={:?}", m.remove(&40), m.remove(&40));
    println!("remove_entry 0={:?}", m.remove_entry(&0));
    println!("pop_first={:?} pop_last={:?}", m.pop_first(), m.pop_last());
    println!("now {:?}", m);
    *m.entry(5).or_insert(String::new()) += "five";
    m.entry(20).and_modify(|v| v.push('#')).or_default();
    m.entry(7).or_insert_with(|| "seven".to_string());
    let ek = *m.entry(99).key();
    println!("entry key={} {:?}", ek, m);
    if let std::collections::btree_map::Entry::Occupied(o) = m.entry(7) {
        println!("occupied {} {}", o.key(), o.get());
        println!("removed {:?}", o.remove());
    }
    m.retain(|k, _| k % 10 == 0);
    println!("retain {:?}", m);
    let mut words: BTreeMap<&str, usize> = BTreeMap::new();
    for w in "one two three two three three".split(' ') { *words.entry(w).or_default() += 1; }
    println!("{:#?}", words);
    let mut lo: BTreeMap<u32, u32> = (0..20).map(|i| (i, i * i)).collect();
    let hi = lo.split_off(&12);
    println!("split lo={:?}", lo.keys().collect::<Vec<_>>());
    println!("split hi={:?}", hi.keys().collect::<Vec<_>>());
    let mut hi2 = hi.clone();
    lo.append(&mut hi2);
    println!("append len={} hi2 empty={} eq={}", lo.len(), hi2.is_empty(), lo == (0..20).map(|i| (i, i * i)).collect());
    let mut overlap: BTreeMap<u32, u32> = [(5, 0), (100, 1)].into();
    lo.append(&mut overlap);
    println!("overlapping append: {:?} {:?} len={}", lo.get(&5), lo.get(&100), lo.len());
    let dup: BTreeMap<u8, char> = vec![(3, 'a'), (1, 'b'), (3, 'c'), (2, 'd'), (1, 'e')].into_iter().collect();
    println!("from_iter keeps last: {:?}", dup);
    println!("cmp: {:?} {:?}", dup.cmp(&dup.clone()), BTreeMap::from([(1u8, 'a')]).cmp(&BTreeMap::from([(1u8, 'b')])));
    let owned: BTreeMap<String, i32> = [("b".to_string(), 2), ("a".to_string(), 1)].into();
    println!("borrowed get={:?} contains={} range={:?}", owned.get("a"), owned.contains_key("b"),
        owned.range::<str, _>((Included("a"), Excluded("b"))).collect::<Vec<_>>());
    let mut sum = 0;
    for (k, v) in &owned { sum += v + k.len() as i32; }
    for (k, v) in owned.clone() { sum += v + k.len() as i32; }
    println!("sum={} into_keys={:?} into_values={:?}", sum, owned.clone().into_keys().collect::<Vec<_>>(), owned.clone().into_values().rev().collect::<Vec<_>>());
    let mut it = owned.iter();
    println!("len={} next={:?} len={} next_back={:?} next={:?}", it.len(), it.next(), it.len(), it.next_back(), it.next());

    // sets
    let mut s: BTreeSet<char> = "hello world".chars().collect();
    println!("{:?} first={:?} last={:?}", s, s.first(), s.last());
    let t: BTreeSet<char> = "lower".chars().collect();
    println!("union={:?}", s.union(&t).collect::<String>());
    println!("inter={:?}", s.intersection(&t).collect::<String>());
    println!("diff={:?}", s.difference(&t).collect::<String>());
    println!("sym={:?}", s.symmetric_difference(&t).collect::<String>());
    println!("ops {:?} {:?}", &s & &t, &s - &t);
    println!("subset={} disjoint={} range={:?}", t.is_subset(&s), s.is_disjoint(&t), s.range('e'..='o').collect::<String>());
    println!("pop {:?} {:?} take={:?} replace={:?}", s.pop_first(), s.pop_last(), s.take(&'l'), s.replace('z'));
    s.retain(|c| *c != 'o');
    println!("{:?} rev={:?}", s, s.iter().rev().collect::<String>());

    // large randomized insert/remove/range vs a sorted Vec model
    let mut big: BTreeMap<u64, u64> = BTreeMap::new();
    let mut model: Vec<(u64, u64)> = Vec::new();
    let mut rng = Rng(12345);
    let mut mism = 0;
    for step in 0..60_000u64 {
        let r = rng.next();
        let k = (r >> 16) % 20_000;
        match r % 5 {
            0 | 1 | 2 => {
                let old = big.insert(k, step);
                let mo = match model.binary_search_by(|e| e.0.cmp(&k)) {
                    Ok(i) => Some(std::mem::replace(&mut model[i].1, step)),
                    Err(i) => { model.insert(i, (k, step)); None }
                };
                if old != mo { mism += 1; }
            }
            3 => {
                let old = big.remove(&k);
                let mo = match model.binary_search_by(|e| e.0.cmp(&k)) {
                    Ok(i) => Some(model.remove(i).1),
                    Err(_) => None,
                };
                if old != mo { mism += 1; }
            }
            _ => {
                let hi = k + (r >> 40) % 500;
                let got: Vec<(u64, u64)> = big.range(k..hi).map(|(a, b)| (*a, *b)).collect();
                let lo_i = model.partition_point(|e| e.0 < k);
                let hi_i = model.partition_point(|e| e.0 < hi);
                if got[..] != model[lo_i..hi_i] { mism += 1; }
            }
        }
    }
    let all: Vec<(u64, u64)> = big.iter().map(|(a, b)| (*a, *b)).collect();
    println!("big len={} model len={} mismatches={} equal={}", big.len(), model.len(), mism, all == model);
    let mut removed = 0;
    while let Some((k, _)) = big.pop_first() {
        if removed % 3 == 0 { big.remove(&(k + 1)); }
        removed += 1;
        if big.len() < 100 { break; }
    }
    println!("drained to {} first={:?}", big.len(), big.first_key_value().map(|x| *x.0 > 0));
}
