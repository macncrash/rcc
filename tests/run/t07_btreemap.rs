use std::collections::BTreeMap;

fn main() {
    let mut m = BTreeMap::new();
    for (i, w) in "delta alpha echo charlie bravo foxtrot".split(' ').enumerate() {
        m.insert(w, i);
    }
    println!("{:?}", m);
    for (k, v) in &m {
        println!("{} => {}", k, v);
    }
    println!("first={:?} last={:?}", m.first_key_value(), m.last_key_value());
    println!("get echo={:?} get zulu={:?}", m.get("echo"), m.get("zulu"));
    *m.get_mut("alpha").unwrap() += 100;
    *m.entry("golf").or_insert(0) += 7;
    *m.entry("alpha").or_insert(0) += 1;
    println!("{:?}", m);
    println!("remove charlie={:?}", m.remove("charlie"));
    println!("keys={:?}", m.keys().collect::<Vec<_>>());
    println!("values={:?}", m.values().collect::<Vec<_>>());
    println!("rev={:?}", m.iter().rev().map(|(k, _)| *k).collect::<Vec<_>>());

    let mut nums: BTreeMap<i32, &str> = BTreeMap::new();
    for i in (0..50).step_by(5) {
        nums.insert(i, if i % 2 == 0 { "even" } else { "odd" });
    }
    println!("range 10..30: {:?}", nums.range(10..30).collect::<Vec<_>>());
    println!("range ..=15: {:?}", nums.range(..=15).map(|(k, _)| k).collect::<Vec<_>>());
    println!("range 33..: {:?}", nums.range(33..).map(|(k, _)| k).collect::<Vec<_>>());
    println!("last below 22: {:?}", nums.range(..22).next_back());
    for (_, v) in nums.range_mut(20..=30) {
        *v = "mid";
    }
    println!("{:?}", nums);
    println!("pop_first={:?} pop_last={:?}", nums.pop_first(), nums.pop_last());
    let high = nums.split_off(&25);
    println!("low={:?}", nums);
    println!("high={:?}", high);
    nums.retain(|k, _| *k != 10);
    println!("retained={:?} len={}", nums, nums.len());
    let mut nested: BTreeMap<u8, BTreeMap<u8, u32>> = BTreeMap::new();
    for i in 0..12u8 {
        *nested.entry(i % 3).or_default().entry(i % 4).or_default() += i as u32;
    }
    println!("{:?}", nested);
    let collected: BTreeMap<char, usize> = "mississippi".chars().fold(BTreeMap::new(), |mut acc, c| {
        *acc.entry(c).or_insert(0) += 1;
        acc
    });
    println!("{:?}", collected);
    let inv: BTreeMap<usize, char> = collected.into_iter().map(|(c, n)| (n, c)).collect();
    println!("{:?}", inv);
}
