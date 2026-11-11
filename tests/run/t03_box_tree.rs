enum Tree {
    Leaf,
    Node(Box<Tree>, i32, Box<Tree>),
}

fn insert(t: Tree, v: i32) -> Tree {
    match t {
        Tree::Leaf => Tree::Node(Box::new(Tree::Leaf), v, Box::new(Tree::Leaf)),
        Tree::Node(l, x, r) => {
            if v < x {
                Tree::Node(Box::new(insert(*l, v)), x, r)
            } else {
                Tree::Node(l, x, Box::new(insert(*r, v)))
            }
        }
    }
}

fn insert_mut(t: &mut Tree, v: i32) {
    match t {
        Tree::Leaf => *t = Tree::Node(Box::new(Tree::Leaf), v, Box::new(Tree::Leaf)),
        Tree::Node(l, x, r) => {
            if v < *x { insert_mut(l, v) } else { insert_mut(r, v) }
        }
    }
}

fn inorder(t: &Tree) {
    if let Tree::Node(l, x, r) = t {
        inorder(l);
        print!("{} ", x);
        inorder(r);
    }
}

fn height(t: &Tree) -> u32 {
    match t {
        Tree::Leaf => 0,
        Tree::Node(l, _, r) => {
            let a = height(l);
            let b = height(r);
            1 + if a > b { a } else { b }
        }
    }
}

fn contains(t: &Tree, v: i32) -> bool {
    match t {
        Tree::Leaf => false,
        Tree::Node(_, x, _) if *x == v => true,
        Tree::Node(l, x, r) => contains(if v < *x { l } else { r }, v),
    }
}

fn min(t: &Tree) -> Option<i32> {
    match t {
        Tree::Leaf => None,
        Tree::Node(l, x, _) => match min(l) {
            Some(m) => Some(m),
            None => Some(*x),
        },
    }
}

fn main() {
    let vals = [50, 30, 70, 20, 40, 60, 80, 35, 45, 65];
    let mut t = Tree::Leaf;
    for i in 0..vals.len() {
        t = insert(t, vals[i]);
    }
    inorder(&t);
    println!();
    println!("height {}", height(&t));
    let vals2 = [35, 36, 80, 10];
    for k in 0..vals2.len() {
        let q = vals2[k];
        println!("contains {} {}", q, contains(&t, q));
    }
    insert_mut(&mut t, 5);
    insert_mut(&mut t, 1);
    insert_mut(&mut t, 99);
    inorder(&t);
    println!();
    println!("height {} min {}", height(&t), min(&t).unwrap());
    println!("{}", min(&Tree::Leaf).is_none());
}
