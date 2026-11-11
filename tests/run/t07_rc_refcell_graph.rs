use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::{Rc, Weak};

struct GNode {
    id: usize,
    succs: Vec<Rc<RefCell<GNode>>>,
    preds: Vec<Weak<RefCell<GNode>>>,
    mark: bool,
}

type N = Rc<RefCell<GNode>>;

fn node(id: usize) -> N {
    Rc::new(RefCell::new(GNode { id, succs: vec![], preds: vec![], mark: false }))
}

fn edge(a: &N, b: &N) {
    a.borrow_mut().succs.push(b.clone());
    b.borrow_mut().preds.push(Rc::downgrade(a));
}

fn dfs(n: &N, order: &mut Vec<usize>) {
    if n.borrow().mark {
        return;
    }
    n.borrow_mut().mark = true;
    let succs: Vec<N> = n.borrow().succs.clone();
    for s in &succs {
        dfs(s, order);
    }
    order.push(n.borrow().id);
}

fn main() {
    let nodes: Vec<N> = (0..7).map(node).collect();
    // a DAG with a diamond and a side branch; 6 unreachable from 0
    for (a, b) in [(0, 1), (0, 2), (1, 3), (2, 3), (3, 4), (2, 5), (6, 4)] {
        edge(&nodes[a], &nodes[b]);
    }
    let mut post = vec![];
    dfs(&nodes[0], &mut post);
    println!("postorder={:?}", post);
    let rpo: Vec<usize> = post.iter().rev().copied().collect();
    println!("rpo={:?}", rpo);
    for n in &nodes {
        let b = n.borrow();
        let preds: BTreeSet<usize> = b.preds.iter().filter_map(|w| w.upgrade()).map(|p| p.borrow().id).collect();
        let succs: Vec<usize> = b.succs.iter().map(|s| s.borrow().id).collect();
        println!("n{} mark={} preds={:?} succs={:?} strong={} weak={}", b.id, b.mark, preds, succs, Rc::strong_count(n), Rc::weak_count(n));
    }
    // rewrite: redirect every edge into 3 to go into 4 instead
    for n in &nodes {
        let mut b = n.borrow_mut();
        for s in b.succs.iter_mut() {
            if s.borrow().id == 3 {
                *s = nodes[4].clone();
            }
        }
    }
    let total_edges: usize = nodes.iter().map(|n| n.borrow().succs.len()).sum();
    println!("edges={} strong(3)={} strong(4)={}", total_edges, Rc::strong_count(&nodes[3]), Rc::strong_count(&nodes[4]));
    for n in &nodes {
        n.borrow_mut().mark = false;
    }
    let mut post2 = vec![];
    dfs(&nodes[0], &mut post2);
    println!("postorder after rewrite={:?}", post2);
}
