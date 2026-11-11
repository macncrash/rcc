use std::collections::{BTreeMap, HashMap};

fn main() {
    let records = [
        ("x86", "add", 3), ("arm", "add", 1), ("x86", "mov", 10), ("riscv", "addi", 4),
        ("arm", "ldr", 6), ("x86", "add", 2), ("arm", "add", 5), ("riscv", "addi", 1),
    ];
    let mut by_arch: HashMap<String, HashMap<String, u32>> = HashMap::new();
    for (arch, op, n) in records {
        *by_arch.entry(arch.to_string()).or_default().entry(op.to_string()).or_insert(0) += n;
    }
    let mut archs: Vec<&String> = by_arch.keys().collect();
    archs.sort();
    for a in archs {
        let ops = &by_arch[a];
        let mut ov: Vec<(&String, &u32)> = ops.iter().collect();
        ov.sort();
        let total: u32 = ops.values().sum();
        println!("{} total={} {:?}", a, total, ov);
    }
    if let Some(inner) = by_arch.get_mut("arm") {
        inner.remove("ldr");
        inner.insert("str".into(), 99);
    }
    let arm: BTreeMap<_, _> = by_arch["arm"].iter().collect();
    println!("arm now {:?}", arm);
    let flat: BTreeMap<(String, String), u32> = by_arch
        .iter()
        .flat_map(|(a, m)| m.iter().map(move |(o, n)| ((a.clone(), o.clone()), *n)))
        .collect();
    for ((a, o), n) in &flat {
        println!("{}/{} = {}", a, o, n);
    }
    let mut index: HashMap<usize, Vec<&str>> = HashMap::new();
    for w in "one three five seven eleven thirteen two".split(' ') {
        index.entry(w.len()).or_insert_with(Vec::new).push(w);
    }
    let mut lens: Vec<_> = index.into_iter().collect();
    lens.sort();
    println!("{:?}", lens);
    let m1: HashMap<i32, i32> = (0..10).map(|i| (i, i * i)).collect();
    let m2: HashMap<i32, i32> = (0..10).rev().map(|i| (i, i * i)).collect();
    println!("maps equal={}", m1 == m2);
    let mut inverted: Vec<(i32, i32)> = m1.iter().map(|(k, v)| (*v, *k)).filter(|(v, _)| v % 2 == 1).collect();
    inverted.sort();
    println!("{:?}", inverted);
}
