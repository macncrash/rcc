use std::collections::HashMap;

fn sorted_pairs(m: &HashMap<String, i32>) -> Vec<(String, i32)> {
    let mut v: Vec<(String, i32)> = m.iter().map(|(k, v)| (k.clone(), *v)).collect();
    v.sort();
    v
}

fn main() {
    let mut m: HashMap<String, i32> = HashMap::new();
    println!("empty len={} is_empty={}", m.len(), m.is_empty());
    for (i, w) in ["apple", "banana", "cherry", "date", "elder"].iter().enumerate() {
        let old = m.insert(w.to_string(), (i as i32) * 10);
        println!("insert {} -> {:?}", w, old);
    }
    let old = m.insert("banana".to_string(), 99);
    println!("re-insert banana -> {:?}", old);
    println!("len={}", m.len());
    println!("get apple={:?} get zzz={:?}", m.get("apple"), m.get("zzz"));
    println!("contains cherry={} contains fig={}", m.contains_key("cherry"), m.contains_key("fig"));
    if let Some(v) = m.get_mut("cherry") {
        *v += 5;
    }
    println!("cherry after get_mut={}", m["cherry"]);
    let r = m.remove("date");
    println!("remove date -> {:?}, again -> {:?}", r, m.remove("date"));
    for (k, v) in sorted_pairs(&m) {
        println!("  {} = {}", k, v);
    }
    let mut keys: Vec<&String> = m.keys().collect();
    keys.sort();
    println!("keys={:?}", keys);
    let mut vals: Vec<i32> = m.values().copied().collect();
    vals.sort();
    println!("values={:?}", vals);
    let total: i32 = m.values().sum();
    println!("sum={}", total);
    for v in m.values_mut() {
        *v *= 2;
    }
    println!("after values_mut: {:?}", sorted_pairs(&m));
    m.retain(|_, v| *v > 50);
    println!("after retain: {:?}", sorted_pairs(&m));
    m.extend(vec![("x".to_string(), 1), ("y".to_string(), 2)]);
    println!("after extend: {:?}", sorted_pairs(&m));
    let removed = m.remove_entry("x");
    println!("remove_entry x -> {:?}", removed);
    m.clear();
    println!("after clear len={}", m.len());

    let mut ints: HashMap<u64, u64> = HashMap::with_capacity(16);
    for i in 0..1000u64 {
        *ints.entry(i % 37).or_insert(0) += i;
    }
    let mut ks: Vec<_> = ints.iter().map(|(k, v)| (*k, *v)).collect();
    ks.sort();
    println!("buckets={} first={:?} last={:?}", ks.len(), ks[0], ks[ks.len() - 1]);
    let chk: u64 = ks.iter().map(|(k, v)| k * 1000 + v).sum();
    println!("checksum={}", chk);
}
