#![allow(dead_code)]
// t12: a control-flow graph built as an Rc<RefCell<Node>> graph (with Weak
// back-edges for predecessors), flattened into the CSR FlatAdj, then
// reverse postorder, Cooper-Harvey-Kennedy dominators, dominance frontiers,
// dominator tree, natural loops and phi placement.
// FlatAdj, compute_reverse_postorder, intersect, compute_dominators,
// compute_dominance_frontiers and build_dom_tree_children are lifted nearly
// verbatim from ccc src/ir/analysis.rs; FxHashSet from src/common/fx_hash.rs.
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::{Rc, Weak};

// ---- lifted from ccc src/common/fx_hash.rs ----------------------------------
pub type FxHashSet<V> = HashSet<V, BuildHasherDefault<FxHasher>>;
const SEED: u64 = 0x517cc1b727220a95;
#[derive(Default)]
pub struct FxHasher {
    hash: u64,
}
impl FxHasher {
    #[inline]
    fn add_to_hash(&mut self, i: u64) {
        self.hash = self.hash.rotate_left(5) ^ i;
        self.hash = self.hash.wrapping_mul(SEED);
    }
}
impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            let word = u64::from_ne_bytes(chunk.try_into().unwrap());
            self.add_to_hash(word);
        }
        let remainder = chunks.remainder();
        if !remainder.is_empty() {
            let mut last = 0u64;
            for (i, &byte) in remainder.iter().enumerate() {
                last |= (byte as u64) << (i * 8);
            }
            self.add_to_hash(last);
        }
    }
    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add_to_hash(i as u64);
    }
    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add_to_hash(i as u64);
    }
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add_to_hash(i);
    }
    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add_to_hash(i as u64);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

// ---- lifted from ccc src/ir/analysis.rs ---------------------------------------
pub struct FlatAdj {
    offsets: Vec<u32>,
    data: Vec<u32>,
}

impl FlatAdj {
    #[inline]
    pub fn row(&self, i: usize) -> &[u32] {
        let start = self.offsets[i] as usize;
        let end = self.offsets[i + 1] as usize;
        &self.data[start..end]
    }
    #[inline]
    pub fn len(&self, i: usize) -> usize {
        (self.offsets[i + 1] - self.offsets[i]) as usize
    }
    fn from_vecs(vecs: Vec<Vec<u32>>) -> Self {
        let n = vecs.len();
        let mut offsets = Vec::with_capacity(n + 1);
        let total: usize = vecs.iter().map(|v| v.len()).sum();
        let mut data = Vec::with_capacity(total);
        let mut offset = 0u32;
        for v in &vecs {
            offsets.push(offset);
            data.extend_from_slice(v);
            offset += v.len() as u32;
        }
        offsets.push(offset);
        FlatAdj { offsets, data }
    }
}

pub fn compute_reverse_postorder(num_blocks: usize, succs: &FlatAdj) -> Vec<usize> {
    let mut visited = vec![false; num_blocks];
    let mut postorder = Vec::with_capacity(num_blocks);
    fn dfs(node: usize, succs: &FlatAdj, visited: &mut Vec<bool>, postorder: &mut Vec<usize>) {
        visited[node] = true;
        for &succ in succs.row(node) {
            let s = succ as usize;
            if !visited[s] {
                dfs(s, succs, visited, postorder);
            }
        }
        postorder.push(node);
    }
    if num_blocks > 0 {
        dfs(0, succs, &mut visited, &mut postorder);
    }
    postorder.reverse();
    postorder
}

fn intersect(mut finger1: usize, mut finger2: usize, idom: &[usize], rpo_number: &[usize]) -> usize {
    while finger1 != finger2 {
        while rpo_number[finger1] > rpo_number[finger2] {
            finger1 = idom[finger1];
        }
        while rpo_number[finger2] > rpo_number[finger1] {
            finger2 = idom[finger2];
        }
    }
    finger1
}

pub fn compute_dominators(num_blocks: usize, preds: &FlatAdj, succs: &FlatAdj) -> Vec<usize> {
    const UNDEF: usize = usize::MAX;
    let rpo = compute_reverse_postorder(num_blocks, succs);
    let mut rpo_number = vec![UNDEF; num_blocks];
    for (order, &block) in rpo.iter().enumerate() {
        rpo_number[block] = order;
    }
    let mut idom = vec![UNDEF; num_blocks];
    if rpo.is_empty() {
        return idom;
    }
    idom[rpo[0]] = rpo[0]; // Entry dominates itself
    let mut changed = true;
    while changed {
        changed = false;
        for &b in rpo.iter().skip(1) {
            if rpo_number[b] == UNDEF {
                continue;
            }
            let mut new_idom = UNDEF;
            for &p in preds.row(b) {
                let p = p as usize;
                if idom[p] != UNDEF {
                    new_idom = p;
                    break;
                }
            }
            if new_idom == UNDEF {
                continue;
            }
            for &p in preds.row(b) {
                let p = p as usize;
                if p == new_idom {
                    continue;
                }
                if idom[p] != UNDEF {
                    new_idom = intersect(new_idom, p, &idom, &rpo_number);
                }
            }
            if idom[b] != new_idom {
                idom[b] = new_idom;
                changed = true;
            }
        }
    }
    idom
}

pub fn compute_dominance_frontiers(num_blocks: usize, preds: &FlatAdj, idom: &[usize]) -> Vec<FxHashSet<usize>> {
    let mut df = vec![FxHashSet::default(); num_blocks];
    for b in 0..num_blocks {
        if preds.len(b) < 2 {
            continue;
        }
        for &p in preds.row(b) {
            let mut runner = p as usize;
            while runner != idom[b] && runner != usize::MAX {
                df[runner].insert(b);
                if runner == idom[runner] {
                    break;
                }
                runner = idom[runner];
            }
        }
    }
    df
}

pub fn build_dom_tree_children(num_blocks: usize, idom: &[usize]) -> Vec<Vec<usize>> {
    let mut children = vec![Vec::new(); num_blocks];
    for b in 1..num_blocks {
        if idom[b] != usize::MAX && idom[b] != b {
            children[idom[b]].push(b);
        }
    }
    children
}

// ---- Rc<RefCell> graph ----------------------------------------------------------

struct Node {
    id: usize,
    name: String,
    succs: Vec<Rc<RefCell<Node>>>,
    preds: Vec<Weak<RefCell<Node>>>,
    defs: Vec<&'static str>,
}

type NodeRef = Rc<RefCell<Node>>;

struct Graph {
    nodes: Vec<NodeRef>,
    by_name: HashMap<String, NodeRef>,
}

impl Graph {
    fn new() -> Self {
        Graph { nodes: Vec::new(), by_name: HashMap::new() }
    }
    fn node(&mut self, name: &str) -> NodeRef {
        if let Some(n) = self.by_name.get(name) {
            return Rc::clone(n);
        }
        let n = Rc::new(RefCell::new(Node { id: self.nodes.len(), name: name.to_string(), succs: Vec::new(), preds: Vec::new(), defs: Vec::new() }));
        self.nodes.push(Rc::clone(&n));
        self.by_name.insert(name.to_string(), Rc::clone(&n));
        n
    }
    fn edge(&mut self, a: &str, b: &str) {
        let na = self.node(a);
        let nb = self.node(b);
        na.borrow_mut().succs.push(Rc::clone(&nb));
        nb.borrow_mut().preds.push(Rc::downgrade(&na));
    }
    fn flatten(&self) -> (FlatAdj, FlatAdj) {
        let mut succs = Vec::new();
        let mut preds = Vec::new();
        for n in &self.nodes {
            let n = n.borrow();
            succs.push(n.succs.iter().map(|s| s.borrow().id as u32).collect());
            preds.push(n.preds.iter().map(|p| p.upgrade().expect("pred alive").borrow().id as u32).collect());
        }
        (FlatAdj::from_vecs(preds), FlatAdj::from_vecs(succs))
    }
    fn name(&self, i: usize) -> String {
        self.nodes[i].borrow().name.clone()
    }
}

fn dominates(idom: &[usize], a: usize, mut b: usize) -> bool {
    loop {
        if a == b {
            return true;
        }
        if idom[b] == b || idom[b] == usize::MAX {
            return false;
        }
        b = idom[b];
    }
}

fn natural_loops(g: &Graph, idom: &[usize], succs: &FlatAdj, preds: &FlatAdj) -> Vec<(usize, BTreeSet<usize>)> {
    let mut loops = Vec::new();
    for b in 0..g.nodes.len() {
        for &s in succs.row(b) {
            let h = s as usize;
            if idom[b] != usize::MAX && dominates(idom, h, b) {
                // back edge b -> h
                let mut body = BTreeSet::new();
                body.insert(h);
                let mut work = vec![b];
                while let Some(x) = work.pop() {
                    if body.insert(x) {
                        for &p in preds.row(x) {
                            work.push(p as usize);
                        }
                    }
                }
                loops.push((h, body));
            }
        }
    }
    loops
}

fn print_tree(g: &Graph, children: &[Vec<usize>], n: usize, depth: usize, out: &mut Vec<String>) {
    out.push(format!("{}{}", "  ".repeat(depth), g.name(n)));
    for &c in &children[n] {
        print_tree(g, children, c, depth + 1, out);
    }
}

fn analyze(title: &str, edges: &[(&str, &str)], defs: &[(&str, &'static str)]) {
    println!("=== {}", title);
    let mut g = Graph::new();
    g.node("entry");
    for (a, b) in edges {
        g.edge(a, b);
    }
    for (n, v) in defs {
        let node = g.node(n);
        node.borrow_mut().defs.push(v);
    }
    let n = g.nodes.len();
    let (preds, succs) = g.flatten();
    for i in 0..n {
        let node = g.nodes[i].borrow();
        let ss: Vec<String> = node.succs.iter().map(|s| s.borrow().name.clone()).collect();
        println!("  {:>2} {:<8} succs={:?} preds={} rc={} weak={}", i, node.name, ss, preds.len(i), Rc::strong_count(&g.nodes[i]), Rc::weak_count(&g.nodes[i]));
    }
    let rpo = compute_reverse_postorder(n, &succs);
    println!("  rpo: {:?}", rpo.iter().map(|&i| g.name(i)).collect::<Vec<_>>());
    let idom = compute_dominators(n, &preds, &succs);
    for i in 0..n {
        let d = if idom[i] == usize::MAX { "<unreachable>".to_string() } else { g.name(idom[i]) };
        println!("  idom({}) = {}", g.name(i), d);
    }
    let df = compute_dominance_frontiers(n, &preds, &idom);
    for i in 0..n {
        let mut f: Vec<usize> = df[i].iter().copied().collect();
        f.sort();
        if !f.is_empty() {
            println!("  DF({}) = {:?}", g.name(i), f.iter().map(|&x| g.name(x)).collect::<Vec<_>>());
        }
    }
    let children = build_dom_tree_children(n, &idom);
    let mut lines = Vec::new();
    print_tree(&g, &children, 0, 0, &mut lines);
    println!("  domtree:");
    for l in lines {
        println!("    {}", l);
    }
    for (h, body) in natural_loops(&g, &idom, &succs, &preds) {
        println!("  loop header {} body {:?}", g.name(h), body.iter().map(|&x| g.name(x)).collect::<Vec<_>>());
    }
    // phi placement: iterated dominance frontier of def sites per variable
    let mut vars: BTreeSet<&'static str> = BTreeSet::new();
    for node in &g.nodes {
        for v in &node.borrow().defs {
            vars.insert(v);
        }
    }
    for v in vars {
        let mut work: Vec<usize> = g.nodes.iter().filter(|n| n.borrow().defs.contains(&v)).map(|n| n.borrow().id).collect();
        let mut has_phi = BTreeSet::new();
        let mut ever = BTreeSet::new();
        ever.extend(work.iter().copied());
        while let Some(b) = work.pop() {
            let mut f: Vec<usize> = df[b].iter().copied().collect();
            f.sort();
            for d in f {
                if has_phi.insert(d) && ever.insert(d) {
                    work.push(d);
                }
            }
        }
        println!("  phi({}) at {:?}", v, has_phi.iter().map(|&x| g.name(x)).collect::<Vec<_>>());
    }
    // break the Rc cycles explicitly so counts can be observed going down
    let first = Rc::clone(&g.nodes[1]);
    let before = Rc::strong_count(&first);
    for node in &g.nodes {
        node.borrow_mut().succs.clear();
    }
    println!("  {} strong count {} -> {}", first.borrow().name, before, Rc::strong_count(&first));
}

fn main() {
    analyze(
        "diamond",
        &[("entry", "a"), ("entry", "b"), ("a", "join"), ("b", "join"), ("join", "exit")],
        &[("a", "x"), ("b", "x"), ("entry", "y")],
    );
    analyze(
        "nested loops",
        &[
            ("entry", "outer"),
            ("outer", "inner"),
            ("inner", "body"),
            ("body", "inner"),
            ("inner", "latch"),
            ("latch", "outer"),
            ("outer", "exit"),
        ],
        &[("entry", "i"), ("latch", "i"), ("entry", "j"), ("body", "j"), ("outer", "j")],
    );
    // the classic example from Cooper-Harvey-Kennedy / Appel
    analyze(
        "appel",
        &[
            ("entry", "b1"),
            ("b1", "b2"),
            ("b1", "b5"),
            ("b2", "b3"),
            ("b2", "b4"),
            ("b3", "b2"),
            ("b4", "b6"),
            ("b5", "b6"),
            ("b5", "b7"),
            ("b6", "b8"),
            ("b7", "b8"),
            ("b8", "b1"),
            ("b8", "exit"),
            ("dead", "b3"),
        ],
        &[("b2", "v"), ("b5", "v"), ("b7", "w"), ("b3", "w")],
    );
    analyze(
        "irreducible",
        &[("entry", "p"), ("entry", "q"), ("p", "q"), ("q", "p"), ("p", "exit"), ("q", "exit")],
        &[("p", "z")],
    );
}
