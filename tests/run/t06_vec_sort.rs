#[derive(Debug, Clone)]
struct P {
    name: &'static str,
    age: u32,
}
fn main() {
    let mut v = vec![5, 3, 9, 1, 3, 7, -2, 0];
    v.sort();
    println!("{:?}", v);
    v.sort_by(|a, b| b.cmp(a));
    println!("desc {:?}", v);
    v.sort_unstable();
    println!("unstable {:?}", v);
    v.sort_by_key(|x: &i32| (x % 3, *x));
    println!("by key {:?}", v);
    let mut ps = vec![
        P { name: "eve", age: 30 },
        P { name: "bob", age: 25 },
        P { name: "amy", age: 30 },
        P { name: "dan", age: 25 },
        P { name: "cat", age: 40 },
    ];
    ps.sort_by_key(|p| p.age);
    for p in &ps {
        println!("{} {}", p.name, p.age);
    }
    ps.sort_by(|a, b| a.age.cmp(&b.age).reverse().then(a.name.cmp(b.name)));
    println!("{:?}", ps.iter().map(|p| p.name).collect::<Vec<_>>());
    let mut fs = vec![2.5, -1.0, 3.25, 0.5];
    fs.sort_by(|a: &f64, b| a.partial_cmp(b).unwrap());
    println!("{:?}", fs);
    let mut ss = vec!["pear", "Apple", "fig", "banana"];
    ss.sort();
    println!("{:?}", ss);
    ss.sort_by_key(|s| s.len());
    println!("{:?}", ss);
    ss.sort_unstable_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    println!("{:?}", ss);
    let mut big: Vec<u32> = (0..50).map(|i| (i * 37 + 11) % 101).collect();
    big.sort_unstable_by_key(|&x| std::cmp::Reverse(x));
    println!("{:?}", &big[..10]);
    println!("is_sorted-ish {}", big.windows(2).all(|w| w[0] >= w[1]));
    let mut tup = vec![(2, 'b'), (1, 'z'), (2, 'a'), (1, 'a')];
    tup.sort();
    println!("{:?}", tup);
}
