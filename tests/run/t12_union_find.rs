#![allow(dead_code)]
// t12: union-find in several styles: an array-based DSU with path
// compression + union by rank, the map-based `find` lifted from ccc
// src/backend/regalloc.rs (copy_groups), a generic DSU over keys, and two
// clients: Hindley-Milner-style type unification with occurs check and
// Kruskal's minimum spanning tree.
use std::collections::{BTreeMap, HashMap};
use std::fmt;

struct Dsu {
    parent: Vec<usize>,
    rank: Vec<u8>,
    size: Vec<usize>,
    sets: usize,
    finds: u64,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Dsu { parent: (0..n).collect(), rank: vec![0; n], size: vec![1; n], sets: n, finds: 0 }
    }
    fn find(&mut self, x: usize) -> usize {
        self.finds += 1;
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        // path compression
        let mut cur = x;
        while self.parent[cur] != root {
            let next = self.parent[cur];
            self.parent[cur] = root;
            cur = next;
        }
        root
    }
    fn union(&mut self, a: usize, b: usize) -> bool {
        let (mut ra, mut rb) = (self.find(a), self.find(b));
        if ra == rb {
            return false;
        }
        if self.rank[ra] < self.rank[rb] {
            std::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb] = ra;
        self.size[ra] += self.size[rb];
        if self.rank[ra] == self.rank[rb] {
            self.rank[ra] += 1;
        }
        self.sets -= 1;
        true
    }
    fn groups(&mut self) -> Vec<Vec<usize>> {
        let mut m: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for i in 0..self.parent.len() {
            let r = self.find(i);
            m.entry(r).or_default().push(i);
        }
        let mut v: Vec<Vec<usize>> = m.into_values().collect();
        v.sort();
        v
    }
}

// ---- lifted from ccc src/backend/regalloc.rs (copy_groups) ------------------
fn find(root: &HashMap<u32, u32>, mut v: u32) -> u32 {
    while let Some(&r) = root.get(&v) {
        if r == v {
            break;
        }
        v = r;
    }
    v
}

/// Coalesce copy-related values into groups unless their live ranges overlap.
fn copy_groups(copies: &[(u32, u32)], ranges: &HashMap<u32, (u32, u32)>, max_group: usize) -> Vec<Vec<u32>> {
    let compatible = |a: u32, b: u32| {
        let (lo1, hi1) = ranges[&a];
        let (lo2, hi2) = ranges[&b];
        hi1 <= lo2 || hi2 <= lo1
    };
    let mut root: HashMap<u32, u32> = HashMap::new();
    let mut members: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(d, s) in copies {
        let (rd, rs) = (find(&root, d), find(&root, s));
        if rd == rs {
            println!("  copy v{} <- v{}: already grouped", d, s);
            continue;
        }
        let md = members.get(&rd).cloned().unwrap_or_else(|| vec![rd]);
        let ms = members.get(&rs).cloned().unwrap_or_else(|| vec![rs]);
        if md.len() + ms.len() > max_group || !md.iter().all(|&a| ms.iter().all(|&b| compatible(a, b))) {
            println!("  copy v{} <- v{}: rejected (sizes {}+{})", d, s, md.len(), ms.len());
            continue;
        }
        root.insert(rs, rd);
        root.entry(rd).or_insert(rd);
        let mut all = md;
        all.extend(ms);
        members.remove(&rs);
        members.insert(rd, all);
        println!("  copy v{} <- v{}: merged into root v{}", d, s, rd);
    }
    let mut out: Vec<Vec<u32>> = members
        .into_values()
        .filter(|l| l.len() > 1)
        .map(|mut l| {
            l.sort();
            l
        })
        .collect();
    out.sort();
    out
}

// ---- generic DSU keyed by any hashable type -------------------------------------
struct KeyedDsu<K> {
    index: HashMap<K, usize>,
    keys: Vec<K>,
    dsu: Dsu,
}

impl<K: std::hash::Hash + Eq + Clone + Ord + fmt::Debug> KeyedDsu<K> {
    fn new() -> Self {
        KeyedDsu { index: HashMap::new(), keys: Vec::new(), dsu: Dsu::new(0) }
    }
    fn id(&mut self, k: &K) -> usize {
        if let Some(&i) = self.index.get(k) {
            return i;
        }
        let i = self.keys.len();
        self.keys.push(k.clone());
        self.index.insert(k.clone(), i);
        self.dsu.parent.push(i);
        self.dsu.rank.push(0);
        self.dsu.size.push(1);
        self.dsu.sets += 1;
        i
    }
    fn union(&mut self, a: &K, b: &K) -> bool {
        let (ia, ib) = (self.id(a), self.id(b));
        self.dsu.union(ia, ib)
    }
    fn same(&mut self, a: &K, b: &K) -> bool {
        let (ia, ib) = (self.id(a), self.id(b));
        self.dsu.find(ia) == self.dsu.find(ib)
    }
    fn classes(&mut self) -> Vec<Vec<K>> {
        let mut out: Vec<Vec<K>> = self.dsu.groups().into_iter().map(|g| {
            let mut v: Vec<K> = g.into_iter().map(|i| self.keys[i].clone()).collect();
            v.sort();
            v
        }).collect();
        out.sort();
        out
    }
}

// ---- type unification ----------------------------------------------------------
#[derive(Clone, Debug, PartialEq)]
enum Ty {
    Var(usize),
    Int,
    Bool,
    Fn(Box<Ty>, Box<Ty>),
    Pair(Box<Ty>, Box<Ty>),
    List(Box<Ty>),
}

struct Unifier {
    dsu: Dsu,
    binding: Vec<Option<Ty>>,
}

#[derive(Debug)]
enum UnifyError {
    Mismatch(String, String),
    Occurs(usize, String),
}

impl Unifier {
    fn new(n: usize) -> Self {
        Unifier { dsu: Dsu::new(n), binding: vec![None; n] }
    }
    fn resolve(&mut self, t: &Ty) -> Ty {
        match t {
            Ty::Var(v) => {
                let r = self.dsu.find(*v);
                match self.binding[r].clone() {
                    Some(b) => self.resolve(&b),
                    None => Ty::Var(r),
                }
            }
            Ty::Fn(a, b) => Ty::Fn(Box::new(self.resolve(a)), Box::new(self.resolve(b))),
            Ty::Pair(a, b) => Ty::Pair(Box::new(self.resolve(a)), Box::new(self.resolve(b))),
            Ty::List(a) => Ty::List(Box::new(self.resolve(a))),
            other => other.clone(),
        }
    }
    fn occurs(&mut self, v: usize, t: &Ty) -> bool {
        match self.resolve(t) {
            Ty::Var(w) => w == v,
            Ty::Fn(a, b) | Ty::Pair(a, b) => self.occurs(v, &a) || self.occurs(v, &b),
            Ty::List(a) => self.occurs(v, &a),
            _ => false,
        }
    }
    fn unify(&mut self, a: &Ty, b: &Ty) -> Result<(), UnifyError> {
        let (a, b) = (self.resolve(a), self.resolve(b));
        match (&a, &b) {
            (Ty::Var(x), Ty::Var(y)) => {
                self.dsu.union(*x, *y);
                Ok(())
            }
            (Ty::Var(x), t) | (t, Ty::Var(x)) => {
                if self.occurs(*x, t) {
                    return Err(UnifyError::Occurs(*x, show(t)));
                }
                let r = self.dsu.find(*x);
                self.binding[r] = Some(t.clone());
                Ok(())
            }
            (Ty::Int, Ty::Int) | (Ty::Bool, Ty::Bool) => Ok(()),
            (Ty::Fn(a1, r1), Ty::Fn(a2, r2)) | (Ty::Pair(a1, r1), Ty::Pair(a2, r2)) => {
                self.unify(a1, a2)?;
                self.unify(r1, r2)
            }
            (Ty::List(x), Ty::List(y)) => self.unify(x, y),
            _ => Err(UnifyError::Mismatch(show(&a), show(&b))),
        }
    }
}

fn show(t: &Ty) -> String {
    match t {
        Ty::Var(v) => format!("t{}", v),
        Ty::Int => "int".into(),
        Ty::Bool => "bool".into(),
        Ty::Fn(a, b) => format!("({} -> {})", show(a), show(b)),
        Ty::Pair(a, b) => format!("({}, {})", show(a), show(b)),
        Ty::List(a) => format!("[{}]", show(a)),
    }
}

fn var(i: usize) -> Ty {
    Ty::Var(i)
}
fn func(a: Ty, b: Ty) -> Ty {
    Ty::Fn(Box::new(a), Box::new(b))
}
fn pair(a: Ty, b: Ty) -> Ty {
    Ty::Pair(Box::new(a), Box::new(b))
}
fn list(a: Ty) -> Ty {
    Ty::List(Box::new(a))
}

fn main() {
    // --- basic DSU
    let mut d = Dsu::new(12);
    for (a, b) in [(0, 1), (2, 3), (1, 3), (4, 5), (6, 7), (5, 7), (8, 8), (9, 10), (0, 2), (11, 4)] {
        let merged = d.union(a, b);
        println!("union({}, {}) -> {} sets={}", a, b, merged, d.sets);
    }
    println!("groups {:?}", d.groups());
    let r0 = d.find(0);
    println!("size of 0's set {} rank {} finds so far {}", d.size[r0], d.rank[r0], d.finds);
    println!("same(1,2)={} same(3,4)={}", d.find(1) == d.find(2), d.find(3) == d.find(4));

    // --- long chain to exercise path compression
    let n = 1000;
    let mut chain = Dsu::new(n);
    for i in 1..n {
        chain.parent[i] = i - 1; // build a degenerate chain directly
    }
    let before = chain.finds;
    let r = chain.find(n - 1);
    let depth_after: usize = (0..n).filter(|&i| chain.parent[i] != r).count();
    println!("chain root {} non-root-parent nodes after one find: {} (finds {})", r, depth_after, chain.finds - before);

    // --- copy coalescing (ccc regalloc style)
    println!("copy groups:");
    let mut ranges = HashMap::new();
    for (v, lo, hi) in [(1u32, 0u32, 3u32), (2, 3, 6), (3, 6, 9), (4, 2, 8), (5, 9, 12), (6, 12, 14), (7, 0, 20)] {
        ranges.insert(v, (lo, hi));
    }
    let groups = copy_groups(&[(2, 1), (3, 2), (4, 3), (5, 3), (6, 5), (6, 1), (7, 6), (5, 2)], &ranges, 4);
    println!("  result {:?}", groups);

    // --- keyed DSU of strings (e.g. alias classes of symbol names)
    let mut k: KeyedDsu<String> = KeyedDsu::new();
    for (a, b) in [("memcpy", "__builtin_memcpy"), ("printf", "__printf_chk"), ("memcpy", "__memcpy_chk"), ("puts", "puts"), ("malloc", "__libc_malloc")] {
        k.union(&a.to_string(), &b.to_string());
    }
    println!("alias classes {:?}", k.classes());
    println!("same memcpy/__memcpy_chk={} same printf/puts={}", k.same(&"__builtin_memcpy".to_string(), &"__memcpy_chk".to_string()), k.same(&"printf".to_string(), &"puts".to_string()));

    // --- type unification
    let mut u = Unifier::new(10);
    let eqs: Vec<(Ty, Ty)> = vec![
        (var(0), func(var(1), var(2))),
        (var(1), Ty::Int),
        (var(2), var(3)),
        (var(3), list(var(4))),
        (var(4), pair(Ty::Bool, var(5))),
        (var(6), var(5)),
        (var(6), Ty::Int),
        (var(7), func(var(7), Ty::Int)),
        (var(8), Ty::Bool),
        (var(8), Ty::Int),
        (func(var(9), var(9)), func(Ty::Int, var(1))),
    ];
    for (a, b) in &eqs {
        let r = u.unify(a, b);
        println!("unify {} = {} : {:?}", show(a), show(b), r);
    }
    for i in 0..10 {
        let t = u.resolve(&var(i));
        println!("  t{} := {}", i, show(&t));
    }

    // --- Kruskal MST
    let names = ["A", "B", "C", "D", "E", "F", "G"];
    let mut edges = vec![
        (7, 0, 1), (5, 0, 3), (8, 1, 2), (9, 1, 3), (7, 1, 4), (5, 2, 4), (15, 3, 4), (6, 3, 5), (8, 4, 5), (9, 4, 6), (11, 5, 6),
    ];
    edges.sort();
    let mut dsu = Dsu::new(names.len());
    let mut total = 0;
    for (w, a, b) in &edges {
        if dsu.union(*a, *b) {
            total += w;
            println!("mst take {}-{} w={}", names[*a], names[*b], w);
        } else {
            println!("mst skip {}-{} w={}", names[*a], names[*b], w);
        }
    }
    println!("mst total {} sets={}", total, dsu.sets);
}
