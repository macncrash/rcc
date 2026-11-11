use std::collections::HashSet;

fn sorted(s: &HashSet<i32>) -> Vec<i32> {
    let mut v: Vec<i32> = s.iter().copied().collect();
    v.sort();
    v
}

fn main() {
    let a: HashSet<i32> = (1..=10).collect();
    let b: HashSet<i32> = (5..15).step_by(2).collect();
    println!("a={:?}", sorted(&a));
    println!("b={:?}", sorted(&b));
    let mut u: Vec<i32> = a.union(&b).copied().collect();
    u.sort();
    println!("union={:?}", u);
    let i: HashSet<i32> = a.intersection(&b).copied().collect();
    println!("intersection={:?}", sorted(&i));
    let d: HashSet<i32> = a.difference(&b).copied().collect();
    println!("a-b={:?}", sorted(&d));
    let sd: HashSet<i32> = a.symmetric_difference(&b).copied().collect();
    println!("sym={:?}", sorted(&sd));
    let bits = &a & &b;
    println!("a&b={:?}", sorted(&bits));
    let bor = &a | &b;
    println!("a|b len={}", bor.len());
    println!("i subset a={} a superset i={} a disjoint b={}", i.is_subset(&a), a.is_superset(&i), a.is_disjoint(&b));
    let neg: HashSet<i32> = [-1, -2].into_iter().collect();
    println!("disjoint neg={}", a.is_disjoint(&neg));

    let mut s = HashSet::new();
    for w in ["x", "y", "x", "z", "y", "w"] {
        let fresh = s.insert(w);
        println!("insert {} fresh={}", w, fresh);
    }
    println!("len={} contains y={} contains q={}", s.len(), s.contains("y"), s.contains("q"));
    println!("remove x={} remove x={}", s.remove("x"), s.remove("x"));
    println!("take z={:?}", s.take("z"));
    println!("get w={:?}", s.get("w"));
    let mut v: Vec<_> = s.iter().collect();
    v.sort();
    println!("{:?}", v);
    let mut nums: HashSet<u32> = (0..50).collect();
    nums.retain(|n| n % 7 == 3);
    let mut nv: Vec<_> = nums.into_iter().collect();
    nv.sort_unstable();
    println!("retained={:?}", nv);
    let dedup: HashSet<String> = "a b c a b d".split(' ').map(String::from).collect();
    let mut dv: Vec<_> = dedup.into_iter().collect();
    dv.sort();
    println!("dedup={:?}", dv);
    let eq1: HashSet<i32> = [1, 2, 3].into_iter().collect();
    let eq2: HashSet<i32> = [3, 2, 1].into_iter().collect();
    println!("eq={}", eq1 == eq2);
}
