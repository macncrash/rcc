#[derive(Debug, Clone, PartialEq)]
struct Item {
    id: u32,
    tag: String,
}
fn main() {
    let v = vec![3, 1, 4, 1, 5];
    let c: Vec<i32> = v.iter().copied().collect();
    let d: Vec<i32> = v.iter().cloned().collect();
    println!("{:?} {:?} {}", c, d, c == d);
    let items = vec![
        Item { id: 1, tag: "a".into() },
        Item { id: 2, tag: "b".into() },
    ];
    let mut copies: Vec<Item> = items.iter().cloned().collect();
    copies[0].tag.push_str("-copy");
    println!("{:?}", items);
    println!("{:?}", copies);
    let max = v.iter().copied().max().unwrap();
    println!("max {}", max);
    let first_big = items.iter().find(|i| i.id > 1).cloned();
    println!("{:?}", first_big);
    let opt: Option<&i32> = v.first();
    println!("{:?} {:?}", opt.copied(), opt.cloned());
    let strs = vec!["x".to_string(), "y".to_string()];
    let joined: String = strs.iter().cloned().collect::<Vec<_>>().concat();
    println!("{}", joined);
    let cloned_vec = v.clone();
    let mut sorted = cloned_vec.to_vec();
    sorted.sort();
    println!("{:?} {:?}", v, sorted);
}
