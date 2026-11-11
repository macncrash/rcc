// HashSet API (results sorted before printing).
use std::collections::HashSet;

fn s<T: Ord + Clone>(it: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut v: Vec<T> = it.into_iter().collect();
    v.sort();
    v
}

fn main() {
    let mut a: HashSet<i32> = (1..=6).collect();
    let b: HashSet<i32> = [4, 5, 6, 7, 8].into_iter().collect();
    println!("insert 3={} insert 10={} len={}", a.insert(3), a.insert(10), a.len());
    println!("contains 10={} remove 10={} remove 10={}", a.contains(&10), a.remove(&10), a.remove(&10));
    println!("union={:?}", s(a.union(&b).copied()));
    println!("intersection={:?}", s(a.intersection(&b).copied()));
    println!("difference={:?}", s(a.difference(&b).copied()));
    println!("b-a={:?}", s(b.difference(&a).copied()));
    println!("symmetric={:?}", s(a.symmetric_difference(&b).copied()));
    println!("ops: | {:?} & {:?} - {:?} ^ {:?}", s(&a | &b), s(&a & &b), s(&a - &b), s(&a ^ &b));
    let small: HashSet<i32> = [2, 3].into();
    println!("subset={} superset={} disjoint={} disjoint2={}", small.is_subset(&a), a.is_superset(&small), small.is_disjoint(&b), a.is_disjoint(&b));
    let mut words: HashSet<String> = HashSet::new();
    for w in "a rose is a rose is a rose".split(' ') { words.insert(w.to_string()); }
    println!("words={:?} has rose={} get={:?}", s(words.iter().cloned()), words.contains("rose"), words.get("is"));
    println!("take={:?} take again={:?}", words.take("is"), words.take("is"));
    println!("replace={:?} replace new={:?}", words.replace("a".to_string()), words.replace("b".to_string()));
    words.retain(|w| w.len() == 1);
    println!("retained={:?}", s(words.iter().cloned()));
    let mut e: HashSet<u8> = HashSet::with_capacity(3);
    e.extend([1u8, 2, 3, 2, 1]);
    e.extend(&[9u8, 8]);
    println!("extend={:?} len={}", s(e.iter().copied()), e.len());
    let drained = s(e.drain());
    println!("drained={:?} after len={} empty={}", drained, e.len(), e.is_empty());
    let c1: HashSet<&str> = ["x", "y"].into_iter().collect();
    let c2: HashSet<&str> = ["y", "x"].into_iter().collect();
    let c3 = c1.clone();
    println!("eq={} eq clone={} ne={}", c1 == c2, c1 == c3, c1 != ["x"].into_iter().collect::<HashSet<&str>>());
    let one: HashSet<(u8, bool)> = [(1, true)].into_iter().collect();
    println!("{:?} {:#?}", one, one);
    let empty: HashSet<u32> = HashSet::default();
    println!("{:?} len={}", empty, empty.len());
    let mut sum = 0;
    for x in &a { sum += x; }
    for x in a.clone() { sum += x; }
    println!("sum={} count={}", sum, a.iter().count());
    let mut v: Vec<i32> = a.into_iter().collect();
    v.sort();
    println!("into_iter sorted={:?}", v);
    let mut big: HashSet<u64> = HashSet::new();
    for i in 0..10000u64 { big.insert(i.wrapping_mul(0x9e3779b97f4a7c15)); }
    for i in 0..5000u64 { big.remove(&i.wrapping_mul(0x9e3779b97f4a7c15)); }
    let ok = (5000..10000u64).all(|i| big.contains(&i.wrapping_mul(0x9e3779b97f4a7c15)));
    println!("big len={} ok={}", big.len(), ok);
}
