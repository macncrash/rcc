use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Task {
    pri: u32,
    name: String,
}

fn dijkstra(adj: &Vec<Vec<(usize, u64)>>, start: usize) -> Vec<u64> {
    let mut dist = vec![u64::MAX; adj.len()];
    let mut heap = BinaryHeap::new();
    dist[start] = 0;
    heap.push(Reverse((0u64, start)));
    while let Some(Reverse((d, u))) = heap.pop() {
        if d > dist[u] {
            continue;
        }
        for &(v, w) in &adj[u] {
            let nd = d + w;
            if nd < dist[v] {
                dist[v] = nd;
                heap.push(Reverse((nd, v)));
            }
        }
    }
    dist
}

fn main() {
    let mut h = BinaryHeap::new();
    for x in [5, 1, 8, 3, 9, 2, 8] {
        h.push(x);
        println!("push {} peek={:?} len={}", x, h.peek(), h.len());
    }
    let mut out = vec![];
    while let Some(x) = h.pop() {
        out.push(x);
    }
    println!("max order {:?}", out);

    let mut mh: BinaryHeap<Reverse<i32>> = vec![5, 1, 8, 3, 9].into_iter().map(Reverse).collect();
    mh.push(Reverse(0));
    print!("min order");
    while let Some(Reverse(x)) = mh.pop() {
        print!(" {}", x);
    }
    println!();

    let mut tasks = BinaryHeap::new();
    tasks.push(Task { pri: 2, name: "write".into() });
    tasks.push(Task { pri: 5, name: "deploy".into() });
    tasks.push(Task { pri: 2, name: "review".into() });
    tasks.push(Task { pri: 1, name: "rest".into() });
    if let Some(mut top) = tasks.peek_mut() {
        top.pri = 0;
    }
    while let Some(t) = tasks.pop() {
        println!("{:?}", t);
    }

    let heap: BinaryHeap<u8> = [4u8, 7, 1].into_iter().collect();
    println!("sorted={:?}", heap.clone().into_sorted_vec());
    let mut v = heap.into_vec();
    v.sort();
    println!("into_vec sorted={:?}", v);

    let edges = [(0, 1, 4), (0, 2, 1), (2, 1, 2), (1, 3, 1), (2, 3, 5), (3, 4, 3)];
    let mut adj = vec![Vec::new(); 6];
    for &(a, b, w) in &edges {
        adj[a].push((b, w));
        adj[b].push((a, w));
    }
    let d = dijkstra(&adj, 0);
    for (i, x) in d.iter().enumerate() {
        if *x == u64::MAX {
            println!("node {} unreachable", i);
        } else {
            println!("node {} dist {}", i, x);
        }
    }
    let mut a = BinaryHeap::from(vec![1, 3]);
    let mut b = BinaryHeap::from(vec![2, 4]);
    a.append(&mut b);
    println!("append len={} b len={} top={:?}", a.len(), b.len(), a.peek());
}
