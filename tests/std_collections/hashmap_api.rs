// HashMap API, iteration-order independent (results are sorted before printing).
use std::collections::hash_map::{Entry, HashMap};
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default, Clone, Copy)]
struct FxHasher {
    hash: u64,
}
const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;
impl FxHasher {
    fn add(&mut self, i: u64) {
        self.hash = (self.hash.rotate_left(5) ^ i).wrapping_mul(SEED);
    }
}
impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.add(b as u64);
        }
    }
    fn write_u8(&mut self, i: u8) { self.add(i as u64) }
    fn write_u32(&mut self, i: u32) { self.add(i as u64) }
    fn write_u64(&mut self, i: u64) { self.add(i) }
    fn write_usize(&mut self, i: usize) { self.add(i as u64) }
    fn finish(&self) -> u64 { self.hash }
}
type FxHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

fn sorted<K: Ord + Clone, V: Clone>(m: impl Iterator<Item = (K, V)>) -> Vec<(K, V)> {
    let mut v: Vec<(K, V)> = m.collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Kind { A, B(u8), C { x: i32 } }

fn main() {
    // basic insert / replace / get / borrowed lookups
    let mut m: HashMap<String, i32> = HashMap::new();
    println!("empty: len={} is_empty={} get={:?}", m.len(), m.is_empty(), m.get("x"));
    for (i, w) in ["alpha", "beta", "gamma", "delta", "beta"].iter().enumerate() {
        let old = m.insert(w.to_string(), i as i32);
        println!("insert {} -> {:?}", w, old);
    }
    println!("len={} beta={:?} m[\"gamma\"]={} contains delta={} contains eps={}",
        m.len(), m.get("beta"), m["gamma"], m.contains_key("delta"), m.contains_key("eps"));
    println!("get_key_value={:?}", m.get_key_value("alpha"));
    if let Some(v) = m.get_mut("alpha") { *v += 100; }
    println!("after get_mut: {:?}", sorted(m.iter().map(|(k, v)| (k.clone(), *v))));
    println!("remove beta={:?} again={:?}", m.remove("beta"), m.remove("beta"));
    println!("remove_entry gamma={:?}", m.remove_entry("gamma"));
    println!("left: {:?}", sorted(m.clone().into_iter()));

    // entry API
    let mut counts: HashMap<char, usize> = HashMap::new();
    for c in "the quick brown fox jumps over the lazy dog".chars().filter(|c| !c.is_whitespace()) {
        *counts.entry(c).or_insert(0) += 1;
    }
    let mut top: Vec<(char, usize)> = counts.iter().map(|(&c, &n)| (c, n)).filter(|&(_, n)| n > 1).collect();
    top.sort();
    println!("repeated letters: {:?}", top);
    let mut e: HashMap<&str, Vec<u32>> = HashMap::new();
    e.entry("a").or_default().push(1);
    e.entry("a").or_default().push(2);
    e.entry("b").or_insert_with(Vec::new).push(3);
    e.entry("c").or_insert_with_key(|k| vec![k.len() as u32]);
    e.entry("a").and_modify(|v| v.push(99)).or_default();
    e.entry("z").and_modify(|v| v.push(99)).or_default();
    println!("entries: {:?}", sorted(e.iter().map(|(k, v)| (*k, v.clone()))));
    match e.entry("b") {
        Entry::Occupied(mut o) => {
            println!("occupied key={} get={:?}", o.key(), o.get());
            o.get_mut().push(4);
            let old = o.insert(vec![7, 7]);
            println!("insert returned {:?}", old);
        }
        Entry::Vacant(_) => println!("vacant?"),
    }
    match e.entry("q") {
        Entry::Occupied(_) => println!("occupied?"),
        Entry::Vacant(v) => {
            println!("vacant key={}", v.key());
            let r = v.insert(vec![5]);
            r.push(6);
        }
    }
    if let Entry::Occupied(o) = e.entry("c") {
        println!("removed c -> {:?}", o.remove());
    }
    if let Entry::Occupied(o) = e.entry("z") {
        println!("remove_entry z -> {:?}", o.remove_entry());
    }
    println!("entry key: {}", e.entry("nope").key());
    println!("entries: {:?}", sorted(e.iter().map(|(k, v)| (*k, v.clone()))));

    // iteration forms
    let mut sq: FxHashMap<u32, u64> = FxHashMap::default();
    for i in 0..20u32 { sq.insert(i, (i as u64) * (i as u64)); }
    let ksum: u32 = sq.keys().sum();
    let vsum: u64 = sq.values().sum();
    println!("keys sum={} values sum={} len={}", ksum, vsum, sq.len());
    for v in sq.values_mut() { *v += 1; }
    for (_, v) in sq.iter_mut() { *v *= 2; }
    for (_, v) in &mut sq { *v -= 2; }
    let mut cnt = 0;
    for (k, v) in &sq { if *v == 2 * (*k as u64) * (*k as u64) { cnt += 1; } }
    println!("consistent={}", cnt);
    sq.retain(|k, _| k % 3 == 0);
    println!("retain: {:?}", sorted(sq.iter().map(|(k, v)| (*k, *v))));
    let mut ks: Vec<u32> = sq.clone().into_keys().collect();
    ks.sort();
    let mut vs: Vec<u64> = sq.clone().into_values().collect();
    vs.sort();
    println!("into_keys={:?} into_values={:?}", ks, vs);
    let mut drained: Vec<(u32, u64)> = sq.drain().collect();
    drained.sort();
    println!("drained {} now len={} empty={}", drained.len(), sq.len(), sq.is_empty());
    sq.insert(5, 5);
    println!("after drain reuse: {:?}", sq);

    // extend / from_iter / from array / equality / clone / default
    let a: HashMap<i32, &str> = [(1, "one"), (2, "two"), (3, "three")].into_iter().collect();
    let b = HashMap::from([(3, "three"), (2, "two"), (1, "one")]);
    let mut c: HashMap<i32, &str> = HashMap::default();
    c.extend(a.iter().map(|(k, v)| (*k, *v)));
    let mut d: HashMap<i32, &str> = HashMap::with_capacity(2);
    d.extend(&a);
    println!("a==b {} a==c {} a==d {} cap>=2 {}", a == b, a == c, a == d, d.capacity() >= 2);
    c.insert(4, "four");
    println!("a==c after insert {} a!=c {}", a == c, a != c);
    let one: HashMap<Kind, u8> = HashMap::from([(Kind::C { x: -1 }, 9)]);
    println!("{:?}", one);
    println!("{:#?}", one);
    let empty: HashMap<u8, u8> = HashMap::new();
    println!("{:?} {:#?}", empty, empty);
    let mut km: HashMap<Kind, usize> = HashMap::new();
    for (i, k) in [Kind::A, Kind::B(1), Kind::B(2), Kind::C { x: 3 }, Kind::A, Kind::B(1)].iter().enumerate() {
        *km.entry(*k).or_default() += i;
    }
    println!("enum keys: {:?}", sorted(km.into_iter()));

    // tuple / slice / &str keys with borrowed lookups
    let mut t: HashMap<(u32, String), bool> = HashMap::new();
    t.insert((1, "x".to_string()), true);
    println!("tuple get={:?}", t.get(&(1, "x".to_string())));
    let mut sl: HashMap<&[u8], usize> = HashMap::new();
    sl.insert(b"abc", 1);
    sl.insert(b"", 2);
    let key: Vec<u8> = vec![b'a', b'b', b'c'];
    println!("slice get={:?} empty={:?}", sl.get(key.as_slice()), sl.get(&b""[..]));
    let mut vk: HashMap<Vec<u8>, u8> = HashMap::new();
    vk.insert(vec![1, 2], 3);
    println!("vec key by slice={:?}", vk.get(&[1u8, 2][..]));
    let owned = String::from("alpha");
    println!("String key by &String={:?}", m.get(&owned));

    // remove everything in an interleaved order, then reuse
    let mut big: FxHashMap<u64, u64> = FxHashMap::with_capacity_and_hasher(4, Default::default());
    for i in 0..1000u64 { big.insert(i * 7919 % 1000, i); }
    let mut removed = 0;
    for i in (0..1000u64).rev().step_by(2) { if big.remove(&i).is_some() { removed += 1; } }
    let mut ok = true;
    for i in 0..1000u64 { if big.contains_key(&i) != (i % 2 == 0) { ok = false; } }
    println!("removed={} len={} ok={}", removed, big.len(), ok);
    big.clear();
    println!("cleared len={} get={:?}", big.len(), big.get(&0));
    big.insert(1, 1);
    println!("reuse len={}", big.len());
}
