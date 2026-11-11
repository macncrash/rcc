#![allow(dead_code)]
// t12: register-allocation-style code: liveness over a linear instruction
// list with loops (backward dataflow over blocks with bitsets), live interval
// construction, interval sort, classic Poletto/Sarkar linear scan with
// spilling, and call-crossing values restricted to callee-saved registers.
// LiveInterval / PhysReg shapes lifted from ccc src/backend/liveness.rs and
// src/backend/regalloc.rs; the FxHasher from src/common/fx_hash.rs.
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

// ---- lifted from ccc src/common/fx_hash.rs ----------------------------------
pub type FxHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;
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
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.add_to_hash(b as u64);
        }
    }
    fn write_u32(&mut self, i: u32) {
        self.add_to_hash(i as u64);
    }
    fn write_u64(&mut self, i: u64) {
        self.add_to_hash(i);
    }
    fn write_usize(&mut self, i: usize) {
        self.add_to_hash(i as u64);
    }
    fn finish(&self) -> u64 {
        self.hash
    }
}

// ---- lifted from ccc src/backend/liveness.rs --------------------------------
/// end = the last point where the value is used
#[derive(Debug, Clone, Copy)]
pub struct LiveInterval {
    pub start: u32,
    pub end: u32,
    pub value_id: u32,
}

/// A physical register assignment. (ccc src/backend/regalloc.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhysReg(pub u8);

// ---- a tiny machine-independent instruction list ----------------------------

#[derive(Debug, Clone)]
enum Inst {
    Def { dest: u32, uses: Vec<u32> },
    Call { dest: Option<u32>, args: Vec<u32> },
    Use { uses: Vec<u32> },
}

struct Block {
    insts: Vec<Inst>,
    succs: Vec<usize>,
}

/// A small dense bitset.
#[derive(Clone, PartialEq, Eq)]
struct BitSet {
    words: Vec<u64>,
}

impl BitSet {
    fn new(n: usize) -> Self {
        BitSet { words: vec![0; (n + 63) / 64] }
    }
    fn insert(&mut self, i: u32) -> bool {
        let (w, b) = ((i / 64) as usize, i % 64);
        let was = self.words[w] & (1 << b) != 0;
        self.words[w] |= 1 << b;
        !was
    }
    fn remove(&mut self, i: u32) {
        self.words[(i / 64) as usize] &= !(1u64 << (i % 64));
    }
    fn contains(&self, i: u32) -> bool {
        self.words[(i / 64) as usize] & (1 << (i % 64)) != 0
    }
    fn union_with(&mut self, o: &BitSet) -> bool {
        let mut changed = false;
        for (a, b) in self.words.iter_mut().zip(&o.words) {
            let n = *a | *b;
            changed |= n != *a;
            *a = n;
        }
        changed
    }
    fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.words.iter().enumerate().flat_map(|(wi, &w)| (0..64u32).filter(move |b| w & (1u64 << b) != 0).map(move |b| wi as u32 * 64 + b))
    }
    fn count(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }
}

impl std::fmt::Debug for BitSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

fn inst_uses(i: &Inst) -> &[u32] {
    match i {
        Inst::Def { uses, .. } | Inst::Use { uses } => uses,
        Inst::Call { args, .. } => args,
    }
}

fn inst_def(i: &Inst) -> Option<u32> {
    match i {
        Inst::Def { dest, .. } => Some(*dest),
        Inst::Call { dest, .. } => *dest,
        Inst::Use { .. } => None,
    }
}

struct Liveness {
    intervals: Vec<LiveInterval>,
    call_points: Vec<u32>,
    live_in: Vec<BitSet>,
    live_out: Vec<BitSet>,
}

fn compute_liveness(blocks: &[Block], nvals: usize) -> Liveness {
    let n = blocks.len();
    let mut gen = vec![BitSet::new(nvals); n];
    let mut kill = vec![BitSet::new(nvals); n];
    for (bi, b) in blocks.iter().enumerate() {
        for i in &b.insts {
            for &u in inst_uses(i) {
                if !kill[bi].contains(u) {
                    gen[bi].insert(u);
                }
            }
            if let Some(d) = inst_def(i) {
                kill[bi].insert(d);
            }
        }
    }
    let mut live_in = vec![BitSet::new(nvals); n];
    let mut live_out = vec![BitSet::new(nvals); n];
    let mut iterations = 0;
    loop {
        iterations += 1;
        let mut changed = false;
        for bi in (0..n).rev() {
            let mut out = BitSet::new(nvals);
            for &s in &blocks[bi].succs {
                out.union_with(&live_in[s]);
            }
            let mut inn = out.clone();
            for k in kill[bi].iter() {
                inn.remove(k);
            }
            inn.union_with(&gen[bi]);
            if inn != live_in[bi] || out != live_out[bi] {
                changed = true;
                live_in[bi] = inn;
                live_out[bi] = out;
            }
        }
        if !changed {
            break;
        }
    }
    println!("liveness converged in {} iterations", iterations);

    // number program points linearly, extend intervals over live-in/out blocks
    let mut start: FxHashMap<u32, u32> = FxHashMap::default();
    let mut end: FxHashMap<u32, u32> = FxHashMap::default();
    let mut call_points = Vec::new();
    let mut point = 0u32;
    let touch = |v: u32, p: u32, start: &mut FxHashMap<u32, u32>, end: &mut FxHashMap<u32, u32>| {
        let s = start.entry(v).or_insert(p);
        *s = (*s).min(p);
        let e = end.entry(v).or_insert(p);
        *e = (*e).max(p);
    };
    for (bi, b) in blocks.iter().enumerate() {
        let block_start = point;
        for v in live_in[bi].iter() {
            touch(v, block_start, &mut start, &mut end);
        }
        for i in &b.insts {
            if matches!(i, Inst::Call { .. }) {
                call_points.push(point);
            }
            for &u in inst_uses(i) {
                touch(u, point, &mut start, &mut end);
            }
            if let Some(d) = inst_def(i) {
                touch(d, point, &mut start, &mut end);
            }
            point += 1;
        }
        let block_end = point.saturating_sub(1);
        for v in live_out[bi].iter() {
            touch(v, block_end, &mut start, &mut end);
        }
    }
    let mut intervals: Vec<LiveInterval> = start.iter().map(|(&v, &s)| LiveInterval { start: s, end: end[&v], value_id: v }).collect();
    // sort by start, then by value id so the order is independent of hashing
    intervals.sort_by(|a, b| a.start.cmp(&b.start).then(a.value_id.cmp(&b.value_id)));
    Liveness { intervals, call_points, live_in, live_out }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Location {
    Reg(PhysReg),
    Stack(u32),
}

struct RegAllocConfig {
    caller_saved: Vec<PhysReg>,
    callee_saved: Vec<PhysReg>,
}

struct RegAllocResult {
    assignments: FxHashMap<u32, Location>,
    used_callee_saved: Vec<PhysReg>,
    spills: u32,
}

fn crosses_call(iv: &LiveInterval, calls: &[u32]) -> bool {
    calls.iter().any(|&c| iv.start < c && c < iv.end)
}

fn linear_scan(lv: &Liveness, cfg: &RegAllocConfig) -> RegAllocResult {
    let mut active: Vec<(LiveInterval, PhysReg)> = Vec::new();
    let mut free_caller: Vec<PhysReg> = cfg.caller_saved.iter().rev().copied().collect();
    let mut free_callee: Vec<PhysReg> = cfg.callee_saved.iter().rev().copied().collect();
    let mut assignments: FxHashMap<u32, Location> = FxHashMap::default();
    let mut used_callee: Vec<PhysReg> = Vec::new();
    let mut next_slot = 0u32;
    let mut spills = 0;
    for iv in &lv.intervals {
        // expire old intervals
        active.sort_by_key(|(a, r)| (a.end, r.0));
        let mut i = 0;
        while i < active.len() {
            if active[i].0.end < iv.start {
                let (old, r) = active.remove(i);
                println!("  expire v{} from r{} at {}", old.value_id, r.0, iv.start);
                if cfg.callee_saved.contains(&r) {
                    free_callee.push(r);
                } else {
                    free_caller.push(r);
                }
            } else {
                i += 1;
            }
        }
        let needs_callee = crosses_call(iv, &lv.call_points);
        let reg = if needs_callee {
            free_callee.pop()
        } else {
            free_caller.pop().or_else(|| free_callee.pop())
        };
        match reg {
            Some(r) => {
                if cfg.callee_saved.contains(&r) && !used_callee.contains(&r) {
                    used_callee.push(r);
                }
                println!("  v{:<2} [{:>2},{:>2}] -> r{}{}", iv.value_id, iv.start, iv.end, r.0, if needs_callee { " (callee-saved)" } else { "" });
                assignments.insert(iv.value_id, Location::Reg(r));
                active.push((*iv, r));
            }
            None => {
                // spill heuristic: the active interval (with a compatible
                // register) that ends last
                let candidate = active
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, r))| !needs_callee || cfg.callee_saved.contains(r))
                    .max_by_key(|(_, (a, _))| (a.end, a.value_id))
                    .map(|(i, _)| i);
                match candidate {
                    Some(ci) if active[ci].0.end > iv.end => {
                        let (victim, r) = active.remove(ci);
                        assignments.insert(victim.value_id, Location::Stack(next_slot));
                        println!("  v{:<2} [{:>2},{:>2}] steals r{} from v{} (spilled to slot {})", iv.value_id, iv.start, iv.end, r.0, victim.value_id, next_slot);
                        next_slot += 1;
                        assignments.insert(iv.value_id, Location::Reg(r));
                        active.push((*iv, r));
                    }
                    _ => {
                        assignments.insert(iv.value_id, Location::Stack(next_slot));
                        println!("  v{:<2} [{:>2},{:>2}] spilled to slot {}", iv.value_id, iv.start, iv.end, next_slot);
                        next_slot += 1;
                    }
                }
                spills += 1;
            }
        }
    }
    used_callee.sort();
    RegAllocResult { assignments, used_callee_saved: used_callee, spills }
}

/// Check: no two values whose intervals overlap share a register.
fn verify(lv: &Liveness, res: &RegAllocResult) -> Result<(), String> {
    for (i, a) in lv.intervals.iter().enumerate() {
        for b in &lv.intervals[i + 1..] {
            let overlap = a.start <= b.end && b.start <= a.end;
            if !overlap {
                continue;
            }
            if let (Some(Location::Reg(ra)), Some(Location::Reg(rb))) = (res.assignments.get(&a.value_id), res.assignments.get(&b.value_id)) {
                if ra == rb {
                    return Err(format!("v{} and v{} both in r{}", a.value_id, b.value_id, ra.0));
                }
            }
        }
    }
    Ok(())
}

fn def(dest: u32, uses: &[u32]) -> Inst {
    Inst::Def { dest, uses: uses.to_vec() }
}

fn run_case(name: &str, blocks: &[Block], nvals: usize, cfg: &RegAllocConfig) {
    println!("== {}", name);
    let lv = compute_liveness(blocks, nvals);
    for (i, (inn, out)) in lv.live_in.iter().zip(&lv.live_out).enumerate() {
        println!("  bb{} in={:?} out={:?} ({} live-out)", i, inn, out, out.count());
    }
    println!("  calls at {:?}", lv.call_points);
    for iv in &lv.intervals {
        println!("  interval v{} = [{}, {}]{}", iv.value_id, iv.start, iv.end, if crosses_call(iv, &lv.call_points) { " crosses-call" } else { "" });
    }
    let res = linear_scan(&lv, cfg);
    let mut pairs: Vec<(u32, Location)> = res.assignments.iter().map(|(&k, &v)| (k, v)).collect();
    pairs.sort_by_key(|p| p.0);
    let mut line = String::new();
    for (v, loc) in &pairs {
        let s = match loc {
            Location::Reg(r) => format!("v{}:r{} ", v, r.0),
            Location::Stack(s) => format!("v{}:[sp+{}] ", v, s * 8),
        };
        line.push_str(&s);
    }
    println!("  result: {}", line.trim_end());
    println!("  spills={} callee-saved used={:?}", res.spills, res.used_callee_saved);
    match verify(&lv, &res) {
        Ok(()) => println!("  verify ok"),
        Err(e) => println!("  verify FAILED: {}", e),
    }
}

fn main() {
    let cfg = RegAllocConfig { caller_saved: vec![PhysReg(8), PhysReg(9), PhysReg(10)], callee_saved: vec![PhysReg(3), PhysReg(12)] };

    // straight line, low pressure
    let straight = vec![Block {
        insts: vec![def(0, &[]), def(1, &[]), def(2, &[0, 1]), def(3, &[2]), def(4, &[3, 0]), Inst::Use { uses: vec![4] }],
        succs: vec![],
    }];
    run_case("straight", &straight, 8, &cfg);

    // high pressure: 8 values all live together
    let mut insts: Vec<Inst> = (0..8).map(|v| def(v, &[])).collect();
    insts.push(def(8, &[0, 1, 2, 3]));
    insts.push(def(9, &[4, 5, 6, 7]));
    insts.push(def(10, &[8, 9]));
    insts.push(Inst::Use { uses: vec![10, 0] });
    run_case("pressure", &[Block { insts, succs: vec![] }], 16, &cfg);

    // loop with a call: v0 (n) and v1 (acc) live across the call
    let looped = vec![
        Block { insts: vec![def(0, &[]), def(1, &[]), def(2, &[])], succs: vec![1] },
        Block { insts: vec![def(3, &[2, 0]), Inst::Call { dest: Some(4), args: vec![2] }, def(5, &[1, 4]), def(1, &[5]), def(2, &[2])], succs: vec![1, 2] },
        Block { insts: vec![def(6, &[1]), Inst::Use { uses: vec![6] }], succs: vec![] },
    ];
    run_case("loop_call", &looped, 8, &cfg);

    // diamond: values flowing around both arms
    let diamond = vec![
        Block { insts: vec![def(0, &[]), def(1, &[0]), def(2, &[0])], succs: vec![1, 2] },
        Block { insts: vec![def(3, &[1]), def(5, &[3, 2])], succs: vec![3] },
        Block { insts: vec![def(4, &[2]), Inst::Call { dest: None, args: vec![4] }, def(5, &[4, 1])], succs: vec![3] },
        Block { insts: vec![def(6, &[5, 0]), Inst::Use { uses: vec![6] }], succs: vec![] },
    ];
    run_case("diamond", &diamond, 8, &cfg);

    // many calls: everything long-lived must take callee-saved or spill
    let mut insts: Vec<Inst> = (0..5).map(|v| def(v, &[])).collect();
    for k in 0..3 {
        insts.push(Inst::Call { dest: Some(5 + k), args: vec![k] });
    }
    insts.push(def(8, &[0, 1, 2, 3, 4, 5, 6, 7]));
    insts.push(Inst::Use { uses: vec![8] });
    run_case("calls", &[Block { insts, succs: vec![] }], 16, &cfg);

    // bitset sanity across word boundary
    let mut bs = BitSet::new(130);
    for v in [0u32, 5, 63, 64, 65, 127, 128, 129] {
        bs.insert(v);
    }
    bs.remove(64);
    let again = bs.insert(5);
    println!("bitset {:?} count={} again={}", bs, bs.count(), again);
}
