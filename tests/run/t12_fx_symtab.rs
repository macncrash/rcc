#![allow(dead_code)]
// t12: the Fx hasher (lifted verbatim from ccc src/common/fx_hash.rs) used as
// a BuildHasherDefault for HashMap/HashSet, exact hash values for a few
// inputs, and the scoped SymbolTable lifted from ccc
// src/common/symbol_table.rs driving a little block-scoped resolver.
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};

/// Type aliases for HashMap/HashSet using FxHash.
pub type FxHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;
pub type FxHashSet<V> = HashSet<V, BuildHasherDefault<FxHasher>>;

const SEED: u64 = 0x517cc1b727220a95;

/// A speedy hash algorithm used within rustc.
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
        // Process 8 bytes at a time
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            let word = u64::from_ne_bytes(chunk.try_into().unwrap());
            self.add_to_hash(word);
        }
        // Handle remaining bytes
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
    fn write_u16(&mut self, i: u16) {
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

// ---- lifted from ccc src/common/symbol_table.rs (CType replaced by a small enum)

#[derive(Debug, Clone, PartialEq)]
pub enum CType {
    Int,
    Long,
    Char,
    Pointer(Box<CType>),
    Array(Box<CType>, Option<usize>),
}

/// Information about a declared symbol.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: CType,
    pub explicit_alignment: Option<usize>,
}

#[derive(Debug)]
struct Scope {
    symbols: FxHashMap<String, Symbol>,
}

impl Scope {
    fn new() -> Self {
        Self { symbols: FxHashMap::default() }
    }
}

/// Scoped symbol table supporting nested lexical scopes.
#[derive(Debug)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self { scopes: vec![Scope::new()] }
    }
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }
    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }
    pub fn declare(&mut self, symbol: Symbol) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.symbols.insert(symbol.name.clone(), symbol);
        }
    }
    /// The depth of the scope declaring `name` nearest in, 0 being the file's.
    pub fn lookup_depth(&self, name: &str) -> Option<usize> {
        self.scopes.iter().rposition(|scope| scope.symbols.contains_key(name))
    }
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.symbols.get(name) {
                return Some(sym);
            }
        }
        None
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

fn fx<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut h = FxHasher::default();
    t.hash(&mut h);
    h.finish()
}

fn ctype_str(t: &CType) -> String {
    match t {
        CType::Int => "int".into(),
        CType::Long => "long".into(),
        CType::Char => "char".into(),
        CType::Pointer(inner) => format!("{}*", ctype_str(inner)),
        CType::Array(inner, Some(n)) => format!("{}[{}]", ctype_str(inner), n),
        CType::Array(inner, None) => format!("{}[]", ctype_str(inner)),
    }
}

#[derive(Debug)]
enum Event {
    Push,
    Pop,
    Decl(&'static str, CType),
    Use(&'static str),
}

fn main() {
    // exact hash values: integers go through write_uN, strings through write + 0xff
    println!("fx(0u64)      = {:#018x}", fx(&0u64));
    println!("fx(1u64)      = {:#018x}", fx(&1u64));
    println!("fx(42u32)     = {:#018x}", fx(&42u32));
    println!("fx(7u8)       = {:#018x}", fx(&7u8));
    println!("fx(usize::MAX)= {:#018x}", fx(&usize::MAX));
    println!("fx((1u32,2u32))= {:#018x}", fx(&(1u32, 2u32)));
    let mut h = FxHasher::default();
    h.write(b"abcdefgh12345");
    println!("raw write 13 bytes = {:#018x}", h.finish());
    let mut h = FxHasher::default();
    h.write(b"");
    println!("raw write 0 bytes = {:#018x}", h.finish());
    // chain property: hashing in two steps equals manual mixing
    let mut a = FxHasher::default();
    a.write_u64(5);
    a.write_u64(9);
    let manual = ((0u64.rotate_left(5) ^ 5).wrapping_mul(SEED).rotate_left(5) ^ 9).wrapping_mul(SEED);
    println!("two-step {:#x} manual {:#x} equal {}", a.finish(), manual, a.finish() == manual);
    let bh: BuildHasherDefault<FxHasher> = BuildHasherDefault::default();
    println!("build_hasher fresh finish = {}", bh.build_hasher().finish());

    // FxHashMap/FxHashSet behaviour (printed sorted)
    let mut counts: FxHashMap<String, usize> = FxHashMap::default();
    let text = "the quick brown fox jumps over the lazy dog the fox barks and the dog runs";
    for w in text.split_whitespace() {
        *counts.entry(w.to_string()).or_insert(0) += 1;
    }
    let mut v: Vec<(&String, &usize)> = counts.iter().collect();
    v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    println!("word counts: {:?}", v);
    println!("len={} get(fox)={:?} get(cat)={:?}", counts.len(), counts.get("fox"), counts.get("cat"));
    let mut set: FxHashSet<u32> = FxHashSet::default();
    for i in 0..200u32 {
        set.insert(i * 7 % 61);
    }
    let mut sv: Vec<u32> = set.iter().copied().collect();
    sv.sort();
    println!("set len {} first {:?} last {:?}", set.len(), &sv[..5], &sv[sv.len() - 3..]);
    let removed: Vec<bool> = [0u32, 7, 100].iter().map(|k| set.remove(k)).collect();
    println!("removed {:?} len now {}", removed, set.len());
    let mut big: FxHashMap<u64, u64> = FxHashMap::default();
    for i in 0..5000u64 {
        big.insert(i.wrapping_mul(0x9e3779b97f4a7c15), i);
    }
    let sum: u64 = big.values().sum();
    let kx = big.keys().fold(0u64, |acc, k| acc ^ k);
    println!("big len {} sum {} key xor {:#x}", big.len(), sum, kx);
    big.retain(|_, v| *v % 3 == 0);
    println!("after retain len {}", big.len());

    // scoped symbol table
    let mut st = SymbolTable::new();
    let events = vec![
        Event::Decl("x", CType::Int),
        Event::Decl("buf", CType::Array(Box::new(CType::Char), Some(64))),
        Event::Push,
        Event::Use("x"),
        Event::Decl("x", CType::Pointer(Box::new(CType::Long))),
        Event::Use("x"),
        Event::Push,
        Event::Decl("y", CType::Array(Box::new(CType::Pointer(Box::new(CType::Int))), None)),
        Event::Use("y"),
        Event::Use("buf"),
        Event::Use("x"),
        Event::Pop,
        Event::Use("y"),
        Event::Pop,
        Event::Use("x"),
        Event::Use("nope"),
    ];
    for e in &events {
        match e {
            Event::Push => {
                st.push_scope();
                println!("push -> depth {}", st.scopes.len() - 1);
            }
            Event::Pop => {
                st.pop_scope();
                println!("pop -> depth {}", st.scopes.len() - 1);
            }
            Event::Decl(n, t) => {
                st.declare(Symbol { name: n.to_string(), ty: t.clone(), explicit_alignment: if *n == "buf" { Some(16) } else { None } });
                println!("declare {} : {}", n, ctype_str(t));
            }
            Event::Use(n) => match st.lookup(n) {
                Some(sym) => println!("use {} -> {} (depth {:?}, align {:?})", n, ctype_str(&sym.ty), st.lookup_depth(n), sym.explicit_alignment),
                None => println!("use {} -> undeclared", n),
            },
        }
    }
    let mut names: Vec<&String> = st.scopes[0].symbols.keys().collect();
    names.sort();
    println!("file scope: {:?}", names);
}
