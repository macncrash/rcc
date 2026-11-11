use std::hash::Hasher;
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
enum Space { A, B }
struct Q { space: Space, v: bool, ignored: u32 }
impl std::hash::Hash for Q {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.space.hash(state);
        self.v.hash(state);
    }
}
struct W(u32);
impl std::fmt::Display for W {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.0.fmt(f) }
}
#[derive(Default)]
struct Fx(u64);
impl Hasher for Fx {
    fn write(&mut self, b: &[u8]) { for x in b { self.0 = self.0.wrapping_mul(31).wrapping_add(*x as u64); } }
    fn finish(&self) -> u64 { self.0 }
}
fn takes(e: &u32) -> u32 { *e + 1 }
fn mantissa(bytes: &[u8; 16]) -> u64 { u64::from_le_bytes(bytes[0..8].try_into().unwrap()) }
fn main() {
    let mut h = Fx::default();
    std::hash::Hash::hash(&Q { space: Space::B, v: true, ignored: 7 }, &mut h);
    let _ = Space::A;
    println!("{}", h.finish() != 0);
    println!("{}", W(42));
    // cast of an if expression
    let n = 5usize;
    let a = if n == 0 { 1 } else { n.next_power_of_two().max(1) } as u32;
    let lanes = 3usize;
    let r = if a > 2 { 2 * lanes } else { lanes } as i64;
    println!("{} {}", a, r);
    // by-value method on an unsized place
    let mut b = [0u8; 16];
    b[1] = 1;
    println!("{}", mantissa(&b));
    // int var behind a reference in arithmetic
    let k = 0x0001_0000_ffff_0002i64;
    let pieces = |skip: u64| (0..4).filter(|i| (k as u64 >> (16 * i)) & 0xffff != skip).count();
    println!("{} {}", pieces(0), pieces(0xffff));
    // &mut key used to index
    let mut v = vec![10u32, 20, 30];
    let mut keys = vec![2usize, 0];
    for key in keys.iter_mut() { let x = v[*key]; v[0] += x; *key += 0; }
    println!("{:?}", v);
    // projection resolved late, then deref-coerced
    let b1: Option<Box<u32>> = Some(Box::new(4));
    let b2: Option<Box<u32>> = None;
    let dims: Vec<&Option<Box<u32>>> = vec![&b1, &b2];
    let mut s = 0;
    for e in dims.iter().copied().flatten() { s += takes(e); }
    println!("{}", s);
    // LUB coercion of if branches
    let names = vec![String::from("abc")];
    for i in 0..2 {
        let nm = if i < names.len() { &names[i] } else { "" };
        println!("[{}] {}", nm, nm.len());
    }
    let normalized;
    let reg = "x5";
    let n2 = 7;
    let shown = if n2 <= 30 { normalized = format!("x{}", n2); &normalized } else { reg };
    println!("{}", shown);
    let m = match n2 { 1 => &names[0], 2 => "two", _ => "many" };
    println!("{}", m);
    println!("{:?}", true.then_some(3));
    println!("{:?} {:?}", false.then(|| 3), true.then(|| "x"));
    let vals = [1i64, -2, 3];
    fn any_neg(it: impl Iterator<Item = i64> + Clone) -> bool { it.clone().any(|v| v < 0) && it.count() == 3 }
    println!("{}", any_neg(vals.iter().copied()));
}
