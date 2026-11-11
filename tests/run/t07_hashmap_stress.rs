// Cross-check HashMap against BTreeMap under many inserts/removes.
use std::collections::{BTreeMap, HashMap};

fn main() {
    let mut h: HashMap<u32, u64> = HashMap::new();
    let mut b: BTreeMap<u32, u64> = BTreeMap::new();
    let mut x: u32 = 2463534242;
    let mut mismatches = 0;
    for step in 0..20000u64 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        let k = x % 512;
        match x % 5 {
            0 | 1 | 2 => {
                let r1 = h.insert(k, step);
                let r2 = b.insert(k, step);
                if r1 != r2 { mismatches += 1; }
            }
            3 => {
                if h.remove(&k) != b.remove(&k) { mismatches += 1; }
            }
            _ => {
                if h.get(&k) != b.get(&k) { mismatches += 1; }
            }
        }
        if step % 4000 == 0 {
            println!("step {} len {} {}", step, h.len(), b.len());
        }
    }
    println!("mismatches={}", mismatches);
    println!("final len {} {}", h.len(), b.len());
    let mut hv: Vec<(u32, u64)> = h.iter().map(|(k, v)| (*k, *v)).collect();
    hv.sort();
    let bv: Vec<(u32, u64)> = b.iter().map(|(k, v)| (*k, *v)).collect();
    println!("contents equal={}", hv == bv);
    let sum: u64 = h.values().sum();
    println!("sum={}", sum);
    println!("first5={:?}", &bv[..5]);
    let mut strs: HashMap<String, usize> = HashMap::new();
    for i in 0..3000usize {
        strs.insert(format!("sym_{}", i * 7 % 1000), i);
    }
    println!("strs len={} sym_0={:?} sym_999={:?} sym_1000={:?}", strs.len(), strs.get("sym_0"), strs.get("sym_999"), strs.get("sym_1000"));
    strs.retain(|k, _| k.ends_with('7'));
    println!("after retain len={}", strs.len());
    h.shrink_to_fit();
    println!("still len={}", h.len());
}
