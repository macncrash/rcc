fn lookup(keys: &[u32], k: u32) -> Option<usize> {
    let mut i = 0;
    while i < keys.len() {
        if keys[i] == k {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn main() {
    let keys = [10, 20, 30];
    let probes = [20, 30, 10, 40, 20];
    for i in 0..probes.len() {
        println!("probe {}", probes[i]);
        let idx = lookup(&keys, probes[i]).unwrap();
        println!("found at {}", idx);
    }
    println!("unreachable");
}
