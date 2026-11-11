use std::collections::{HashMap, VecDeque};

struct Lru {
    cap: usize,
    map: HashMap<u32, String>,
    order: VecDeque<u32>,
    hits: u32,
    misses: u32,
}

impl Lru {
    fn new(cap: usize) -> Self {
        Lru { cap, map: HashMap::new(), order: VecDeque::new(), hits: 0, misses: 0 }
    }
    fn touch(&mut self, k: u32) {
        if let Some(pos) = self.order.iter().position(|&x| x == k) {
            self.order.remove(pos);
        }
        self.order.push_back(k);
    }
    fn get(&mut self, k: u32) -> Option<&String> {
        if self.map.contains_key(&k) {
            self.hits += 1;
            self.touch(k);
            self.map.get(&k)
        } else {
            self.misses += 1;
            None
        }
    }
    fn put(&mut self, k: u32, v: String) -> Option<u32> {
        let mut evicted = None;
        if !self.map.contains_key(&k) && self.map.len() == self.cap {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
                evicted = Some(old);
            }
        }
        self.map.insert(k, v);
        self.touch(k);
        evicted
    }
}

fn main() {
    let mut c = Lru::new(3);
    let ops: [(char, u32); 14] = [
        ('p', 1), ('p', 2), ('p', 3), ('g', 1), ('p', 4), ('g', 2), ('g', 3),
        ('p', 5), ('g', 1), ('g', 4), ('p', 3), ('g', 5), ('p', 6), ('g', 3),
    ];
    for (op, k) in ops {
        match op {
            'p' => {
                let ev = c.put(k, format!("v{}", k * 100));
                println!("put {} evicted={:?} order={:?}", k, ev, c.order);
            }
            _ => {
                let got = c.get(k).cloned();
                println!("get {} -> {:?} order={:?}", k, got, c.order);
            }
        }
    }
    println!("hits={} misses={}", c.hits, c.misses);
    let mut keys: Vec<_> = c.map.keys().collect();
    keys.sort();
    println!("final keys={:?}", keys);
}
