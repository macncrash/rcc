use std::collections::BTreeMap;

// map from start address to (end, name); look up which region contains an addr
struct Regions {
    m: BTreeMap<u64, (u64, String)>,
}

impl Regions {
    fn add(&mut self, start: u64, len: u64, name: &str) -> bool {
        let end = start + len;
        if let Some((_, (pend, _))) = self.m.range(..end).next_back() {
            if *pend > start {
                return false;
            }
        }
        self.m.insert(start, (end, name.to_string()));
        true
    }
    fn find(&self, addr: u64) -> Option<&str> {
        let (s, (e, n)) = self.m.range(..=addr).next_back()?;
        if addr >= *s && addr < *e { Some(n.as_str()) } else { None }
    }
}

fn main() {
    let mut r = Regions { m: BTreeMap::new() };
    for (s, l, n) in [(0x1000, 0x200, ".text"), (0x1200, 0x80, ".rodata"), (0x1100, 0x10, "overlap"),
                      (0x2000, 0x100, ".data"), (0x1fff, 1, ".pad"), (0x2050, 4, "inside")] {
        println!("add {:#x}+{:#x} {} -> {}", s, l, n, r.add(s, l, n));
    }
    for a in [0x0fff, 0x1000, 0x11ff, 0x1200, 0x127f, 0x1280, 0x1fff, 0x2000, 0x20ff, 0x2100] {
        println!("find {:#x} = {:?}", a, r.find(a));
    }
    for (s, (e, n)) in &r.m {
        println!("{:#06x}..{:#06x} {}", s, e, n);
    }
    let mut hist: BTreeMap<i32, u32> = BTreeMap::new();
    let mut x: i32 = 17;
    for _ in 0..200 {
        x = (x.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fff;
        *hist.entry(x / 4096).or_insert(0) += 1;
    }
    println!("{:?}", hist);
    let below: u32 = hist.range(..4).map(|(_, v)| *v).sum();
    let above: u32 = hist.range(4..).map(|(_, v)| *v).sum();
    println!("below={} above={} total={}", below, above, below + above);
    let mut e = hist.entry(100);
    e = match e {
        std::collections::btree_map::Entry::Vacant(v) => {
            v.insert(1);
            hist.entry(100)
        }
        o => o,
    };
    println!("entry key={}", e.key());
    println!("contains 100={} len={}", hist.contains_key(&100), hist.len());
}
