// Fx hasher as used by rustc / ccc, plugged in via BuildHasherDefault.
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hash, Hasher};

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

#[derive(Default, Clone, Copy)]
pub struct FxHasher {
    hash: u64,
}

impl FxHasher {
    #[inline]
    fn add_to_hash(&mut self, i: u64) {
        self.hash = (self.hash.rotate_left(5) ^ i).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut b = bytes;
        while b.len() >= 8 {
            self.add_to_hash(u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]));
            b = &b[8..];
        }
        if b.len() >= 4 {
            self.add_to_hash(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as u64);
            b = &b[4..];
        }
        for &byte in b {
            self.add_to_hash(byte as u64);
        }
    }
    fn write_u8(&mut self, i: u8) { self.add_to_hash(i as u64); }
    fn write_u16(&mut self, i: u16) { self.add_to_hash(i as u64); }
    fn write_u32(&mut self, i: u32) { self.add_to_hash(i as u64); }
    fn write_u64(&mut self, i: u64) { self.add_to_hash(i); }
    fn write_usize(&mut self, i: usize) { self.add_to_hash(i as u64); }
    fn finish(&self) -> u64 { self.hash }
}

pub type FxBuildHasher = BuildHasherDefault<FxHasher>;
pub type FxHashMap<K, V> = HashMap<K, V, FxBuildHasher>;
pub type FxHashSet<K> = HashSet<K, FxBuildHasher>;

fn fx<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut h = FxHasher::default();
    t.hash(&mut h);
    h.finish()
}

fn main() {
    // the hash values themselves are deterministic
    println!("u32 7: {:016x}", fx(&7u32));
    println!("u64 7: {:016x}", fx(&7u64));
    println!("u8 255: {:016x}", fx(&255u8));
    println!("bytes: {:016x}", { let mut h = FxHasher::default(); h.write(b"hello world!!"); h.finish() });
    println!("tuple: {:016x}", fx(&(1u32, 2u16, 3u8)));

    let mut m: FxHashMap<String, usize> = FxHashMap::default();
    for (i, name) in ["main", "foo", "bar", "baz", "foo", "main", "qux"].iter().enumerate() {
        let len = m.len();
        let id = *m.entry(name.to_string()).or_insert(len);
        println!("{} at {} -> id {}", name, i, id);
    }
    let mut v: Vec<_> = m.iter().map(|(k, v)| (*v, k.clone())).collect();
    v.sort();
    println!("{:?}", v);

    let mut s: FxHashSet<u32> = FxHashSet::default();
    for i in 0..200u32 {
        s.insert(i.wrapping_mul(2654435761) % 97);
    }
    let mut sv: Vec<_> = s.iter().copied().collect();
    sv.sort();
    println!("set len={} min={} max={}", sv.len(), sv[0], sv[sv.len() - 1]);
    let mut cap: FxHashMap<u64, u64> = HashMap::with_capacity_and_hasher(8, Default::default());
    for i in 0..20 {
        cap.insert(i, i * i);
    }
    println!("cap get 19 = {:?}, get 20 = {:?}", cap.get(&19), cap.get(&20));
}
