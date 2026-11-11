// Closures as comparators and predicates in sort_by, sort_by_key, retain,
// binary_search_by, max_by_key, capturing configuration.
#[derive(Debug, Clone)]
struct Item {
    name: &'static str,
    price: u32,
    qty: u32,
}

fn main() {
    let mut items = vec![
        Item { name: "pear", price: 30, qty: 4 },
        Item { name: "apple", price: 20, qty: 10 },
        Item { name: "fig", price: 90, qty: 1 },
        Item { name: "kiwi", price: 20, qty: 3 },
        Item { name: "date", price: 55, qty: 7 },
    ];
    items.sort_by(|a, b| a.price.cmp(&b.price).then(b.qty.cmp(&a.qty)));
    for i in &items {
        println!("{} {} {}", i.name, i.price, i.qty);
    }
    items.sort_by_key(|i| i.name);
    println!("{:?}", items.iter().map(|i| i.name).collect::<Vec<_>>());
    let descending = true;
    items.sort_by(|a, b| {
        let o = (a.price * a.qty).cmp(&(b.price * b.qty));
        if descending { o.reverse() } else { o }
    });
    println!("{:?}", items.iter().map(|i| i.price * i.qty).collect::<Vec<_>>());
    let best = items.iter().max_by_key(|i| i.qty).unwrap();
    println!("max qty {}", best.name);
    let cheapest = items.iter().min_by(|a, b| a.price.cmp(&b.price)).unwrap();
    println!("cheapest {}", cheapest.name);
    let limit = 40;
    let mut affordable = items.clone();
    affordable.retain(|i| i.price <= limit);
    println!("{}", affordable.len());
    let mut nums = vec![5, 3, 9, 1, 7];
    nums.sort_by(|a, b| b.cmp(a));
    println!("{:?}", nums);
    let target = 3;
    println!("{:?}", nums.binary_search_by(|p| target.cmp(p)));
    println!("{:?}", nums.binary_search_by(|p| 4.cmp(p)));
    let pos = nums.iter().position(|&x| x < 5);
    println!("{:?}", pos);
    let mut strs = vec!["bb", "a", "ccc", "dd"];
    strs.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| b.cmp(a)));
    println!("{:?}", strs);
}
