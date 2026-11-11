fn main() {
    let mut v: Vec<i32> = Vec::new();
    println!("empty len={} is_empty={}", v.len(), v.is_empty());
    for i in 0..10 {
        v.push(i * i);
        println!("push {} -> len {} last {:?}", i * i, v.len(), v.last());
    }
    println!("{:?}", v);
    while let Some(x) = v.pop() {
        if x < 10 {
            println!("stop at {}", x);
            break;
        }
        println!("popped {}", x);
    }
    println!("rest {:?} len {}", v, v.len());
    let mut w = Vec::with_capacity(4);
    println!("cap>=4 {}", w.capacity() >= 4);
    w.push(String::from("a"));
    w.push("b".to_string());
    w.push(format!("c{}", 3));
    println!("{:?}", w);
    let s = w.pop().unwrap();
    println!("popped string {} len {}", s, w.len());
    let mut e: Vec<u8> = vec![];
    println!("pop empty {:?}", e.pop());
    e.push(255);
    println!("{:?} first {:?} last {:?}", e, e.first(), e.last());
    let v2 = vec![7u64; 5];
    println!("{:?} sum {}", v2, v2.iter().sum::<u64>());
    let mut v3 = vec![1, 2, 3];
    v3.clear();
    println!("cleared {:?} {}", v3, v3.is_empty());
    let nested = vec![vec![1, 2], vec![], vec![3]];
    println!("{:?} total {}", nested, nested.iter().map(|x| x.len()).sum::<usize>());
}
