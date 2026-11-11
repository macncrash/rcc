use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Reg {
    Rax,
    Rbx,
    R(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Key {
    module: String,
    id: u32,
}

fn main() {
    let mut uses: HashMap<Reg, Vec<usize>> = HashMap::new();
    let seq = [Reg::Rax, Reg::R(8), Reg::Rbx, Reg::Rax, Reg::R(9), Reg::R(8), Reg::Rax];
    for (i, r) in seq.iter().enumerate() {
        uses.entry(*r).or_default().push(i);
    }
    let mut ks: Vec<_> = uses.keys().copied().collect();
    ks.sort();
    for k in &ks {
        println!("{:?} used at {:?}", k, uses[k]);
    }
    let mut pairs: HashMap<(i32, i32), &str> = HashMap::new();
    pairs.insert((0, 0), "origin");
    pairs.insert((1, -1), "se");
    pairs.insert((-1, 1), "nw");
    println!("{:?} {:?} {:?}", pairs.get(&(0, 0)), pairs.get(&(1, -1)), pairs.get(&(2, 2)));

    let mut table: HashMap<Key, f64> = HashMap::new();
    for (m, id, v) in [("a", 1, 1.5), ("b", 1, 2.5), ("a", 2, 3.0), ("a", 1, 9.0)] {
        table.insert(Key { module: m.to_string(), id }, v);
    }
    let mut tk: Vec<_> = table.iter().collect();
    tk.sort_by(|x, y| x.0.cmp(y.0));
    for (k, v) in tk {
        println!("{}::{} = {}", k.module, k.id, v);
    }
    let probe = Key { module: "a".into(), id: 2 };
    println!("probe {:?}", table.get(&probe));

    let mut seen: HashSet<Vec<u8>> = HashSet::new();
    for w in ["ab", "ba", "ab", "abc", ""] {
        let mut b = w.as_bytes().to_vec();
        b.sort();
        println!("{:?} new={}", w, seen.insert(b));
    }
    let mut strs: HashMap<String, HashSet<char>> = HashMap::new();
    for w in ["hello", "world", "help"] {
        strs.insert(w.to_string(), w.chars().collect());
    }
    let mut names: Vec<&String> = strs.keys().collect();
    names.sort();
    for n in names {
        let mut cs: Vec<char> = strs[n].iter().copied().collect();
        cs.sort();
        let s: String = cs.into_iter().collect();
        println!("{} -> {}", n, s);
    }
    let common: HashSet<char> = strs["hello"].intersection(&strs["help"]).copied().collect();
    let mut cv: Vec<_> = common.into_iter().collect();
    cv.sort();
    println!("common={:?}", cv);
    let opt_keys: HashMap<Option<u8>, u8> = [(None, 0), (Some(1), 1), (Some(2), 4)].into_iter().collect();
    println!("{:?} {:?}", opt_keys.get(&None), opt_keys.get(&Some(2)));
}
