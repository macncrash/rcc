// Arena-style tree without Box/Vec: nodes stored in a fixed array, children by index.
enum Node {
    Leaf(i64),
    Add(usize, usize),
    Mul(usize, usize),
    Neg(usize),
}

fn eval(nodes: &[Node; 9], i: usize, depth: u32) -> i64 {
    let v = match nodes[i] {
        Node::Leaf(v) => v,
        Node::Add(a, b) => eval(nodes, a, depth + 1) + eval(nodes, b, depth + 1),
        Node::Mul(a, b) => eval(nodes, a, depth + 1) * eval(nodes, b, depth + 1),
        Node::Neg(a) => -eval(nodes, a, depth + 1),
    };
    println!("node {} depth {} = {}", i, depth, v);
    v
}

fn count_leaves(nodes: &[Node; 9], i: usize) -> u32 {
    match &nodes[i] {
        Node::Leaf(_) => 1,
        Node::Add(a, b) | Node::Mul(a, b) => count_leaves(nodes, *a) + count_leaves(nodes, *b),
        Node::Neg(a) => count_leaves(nodes, *a),
    }
}

fn main() {
    // (2 + 3) * -(4 + (5 * 6))
    let nodes = [
        Node::Leaf(2),
        Node::Leaf(3),
        Node::Add(0, 1),
        Node::Leaf(4),
        Node::Leaf(5),
        Node::Leaf(6),
        Node::Mul(4, 5),
        Node::Add(3, 6),
        Node::Neg(7),
    ];
    let left = eval(&nodes, 2, 0);
    let right = eval(&nodes, 8, 0);
    println!("result {}", left * right);
    println!("leaves {} {}", count_leaves(&nodes, 2), count_leaves(&nodes, 8));
    if let Node::Mul(x, y) = nodes[6] {
        println!("mul children {} {}", x, y);
    }
}
