// 100k-scale inserts/removes checked against a simple model, with the
// default hasher, an Fx hasher and a pathological identity hasher.
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};

#[derive(Default, Clone, Copy)]
struct FxHasher { hash: u64 }
impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes { self.hash = (self.hash.rotate_left(5) ^ b as u64).wrapping_mul(0x517cc1b727220a95); }
    }
    fn write_u64(&mut self, i: u64) { self.hash = (self.hash.rotate_left(5) ^ i).wrapping_mul(0x517cc1b727220a95); }
    fn finish(&self) -> u64 { self.hash }
}
/// Identity hash: every key's hash is its value (worst case for weak mixing).
#[derive(Default, Clone, Copy)]
struct IdHasher { hash: u64 }
impl Hasher for IdHasher {
    fn write(&mut self, bytes: &[u8]) { for &b in bytes { self.hash = (self.hash << 8) | b as u64; } }
    fn write_u64(&mut self, i: u64) { self.hash = i; }
    fn finish(&self) -> u64 { self.hash }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn run<S: BuildHasher + Default>(name: &str, n: usize, keyspace: u64, stride: u64) {
    let mut m: HashMap<u64, u64, S> = HashMap::default();
    let mut model: Vec<Option<u64>> = vec![None; keyspace as usize];
    let mut rng = Rng(0x9e3779b97f4a7c15);
    let mut hits = 0u64;
    let mut mismatches = 0u64;
    for step in 0..n {
        let r = rng.next();
        let k = (r >> 8) % keyspace;
        let key = k * stride;
        match r % 8 {
            0..=3 => {
                let old = m.insert(key, step as u64);
                if old != model[k as usize] { mismatches += 1; }
                model[k as usize] = Some(step as u64);
            }
            4 | 5 => {
                let old = m.remove(&key);
                if old != model[k as usize] { mismatches += 1; }
                model[k as usize] = None;
            }
            6 => {
                let got = m.get(&key).copied();
                if got != model[k as usize] { mismatches += 1; }
                if got.is_some() { hits += 1; }
            }
            _ => {
                let e = m.entry(key).or_insert(7);
                *e += 1;
                let mv = model[k as usize].unwrap_or(7) + 1;
                model[k as usize] = Some(mv);
            }
        }
    }
    let live = model.iter().filter(|x| x.is_some()).count();
    let mut sum_m = 0u64;
    for (k, v) in &m {
        if model[(*k / stride) as usize] != Some(*v) { mismatches += 1; }
        sum_m = sum_m.wrapping_add(*v);
    }
    let sum_model: u64 = model.iter().flatten().fold(0u64, |a, b| a.wrapping_add(*b));
    println!("{}: len={} live={} hits={} mismatches={} sums_equal={}", name, m.len(), live, hits, mismatches, sum_m == sum_model);
    // remove everything through retain, then check emptiness
    m.retain(|k, _| k % 2 == 1);
    let odd = model.iter().enumerate().filter(|(i, x)| x.is_some() && ((*i as u64) * stride) % 2 == 1).count();
    println!("{}: after retain len={} expected={}", name, m.len(), odd);
}

fn main() {
    run::<std::collections::hash_map::RandomState>("sip", 100_000, 20_000, 1);
    run::<BuildHasherDefault<FxHasher>>("fx", 100_000, 20_000, 1);
    run::<BuildHasherDefault<FxHasher>>("fx-stride", 100_000, 20_000, 4096);
    run::<BuildHasherDefault<IdHasher>>("id", 100_000, 20_000, 1);
    run::<BuildHasherDefault<IdHasher>>("id-stride", 100_000, 20_000, 1 << 20);

    // grow from empty one at a time; then shrink to empty
    let mut s: HashSet<String> = HashSet::new();
    for i in 0..50_000 { s.insert(format!("k{}", i)); }
    let mut all = true;
    for i in 0..50_000 { if !s.contains(format!("k{}", i).as_str()) { all = false; } }
    for i in 0..50_000 { if i % 5 != 0 { s.remove(&format!("k{}", i)); } }
    println!("strings: len={} all_found={} k5={} k6={}", s.len(), all, s.contains("k5"), s.contains("k6"));
    for i in 0..50_000 { s.remove(&format!("k{}", i)); }
    println!("strings: emptied len={}", s.len());
    let h = std::hash::BuildHasherDefault::<FxHasher>::default();
    let mut hh = h.build_hasher();
    "abc".hash(&mut hh);
    println!("fx(abc)={:x}", hh.finish());
}
