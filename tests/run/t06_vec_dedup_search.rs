fn main() {
    let mut v = vec![1, 1, 2, 3, 3, 3, 4, 1, 1, 5];
    v.dedup();
    println!("dedup {:?}", v);
    let mut w = vec!["a", "A", "b", "B", "b", "c"];
    w.dedup_by_key(|s| s.to_lowercase());
    println!("dedup_by_key {:?}", w);
    let mut x = vec![1, 2, 4, 5, 7, 8, 10];
    x.dedup_by(|a, b| *a - *b == 1);
    println!("dedup_by {:?}", x);
    let sorted = vec![1, 3, 5, 7, 9, 11, 13];
    for t in [0, 1, 4, 7, 13, 14] {
        println!("search {} -> {:?}", t, sorted.binary_search(&t));
    }
    let r = sorted.binary_search_by(|p| p.cmp(&6));
    println!("by {:?}", r);
    let pairs = vec![(1, "one"), (3, "three"), (5, "five")];
    println!("by_key {:?}", pairs.binary_search_by_key(&3, |&(k, _)| k));
    println!("by_key {:?}", pairs.binary_search_by_key(&4, |&(k, _)| k));
    let mut ins = vec![10, 20, 30];
    for n in [25, 5, 35, 20] {
        let pos = match ins.binary_search(&n) {
            Ok(p) => p,
            Err(p) => p,
        };
        ins.insert(pos, n);
        println!("insert {} at {} -> {:?}", n, pos, ins);
    }
    println!("contains 25 {} contains 26 {}", ins.contains(&25), ins.contains(&26));
    println!("pp {}", ins.partition_point(|&x| x < 21));
}
