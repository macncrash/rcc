use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[derive(Debug)]
struct TreeNode {
    value: i32,
    parent: RefCell<Weak<TreeNode>>,
    children: RefCell<Vec<Rc<TreeNode>>>,
}

fn new_node(v: i32) -> Rc<TreeNode> {
    Rc::new(TreeNode { value: v, parent: RefCell::new(Weak::new()), children: RefCell::new(vec![]) })
}

fn add_child(p: &Rc<TreeNode>, c: Rc<TreeNode>) {
    *c.parent.borrow_mut() = Rc::downgrade(p);
    p.children.borrow_mut().push(c);
}

fn path_to_root(n: &Rc<TreeNode>) -> Vec<i32> {
    let mut out = vec![n.value];
    let mut cur = n.parent.borrow().upgrade();
    while let Some(p) = cur {
        out.push(p.value);
        cur = p.parent.borrow().upgrade();
    }
    out
}

fn sum(n: &Rc<TreeNode>) -> i32 {
    n.value + n.children.borrow().iter().map(sum).sum::<i32>()
}

fn main() {
    let leaf = new_node(3);
    println!("leaf parent = {:?}", leaf.parent.borrow().upgrade().map(|p| p.value));
    println!("leaf strong={} weak={}", Rc::strong_count(&leaf), Rc::weak_count(&leaf));
    {
        let branch = new_node(5);
        add_child(&branch, leaf.clone());
        println!("leaf parent = {:?}", leaf.parent.borrow().upgrade().map(|p| p.value));
        println!("branch strong={} weak={}", Rc::strong_count(&branch), Rc::weak_count(&branch));
        println!("leaf strong={} weak={}", Rc::strong_count(&leaf), Rc::weak_count(&leaf));
        let root = new_node(1);
        add_child(&root, branch.clone());
        let other = new_node(7);
        add_child(&root, other.clone());
        add_child(&other, new_node(11));
        let deep = other.children.borrow()[0].clone();
        println!("path deep={:?} path leaf={:?}", path_to_root(&deep), path_to_root(&leaf));
        println!("sum root={} sum branch={}", sum(&root), sum(&branch));
        println!("root weak={} strong={}", Rc::weak_count(&root), Rc::strong_count(&root));
    }
    println!("after scope leaf parent = {:?}", leaf.parent.borrow().upgrade().map(|p| p.value));
    println!("leaf strong={} weak={}", Rc::strong_count(&leaf), Rc::weak_count(&leaf));

    let w: Weak<i32>;
    {
        let s = Rc::new(99);
        w = Rc::downgrade(&s);
        let w2 = w.clone();
        println!("upgrade={:?} strong={} weak={}", w.upgrade(), w.strong_count(), w2.weak_count());
    }
    println!("after drop upgrade={:?} strong={}", w.upgrade(), w.strong_count());
    let empty: Weak<String> = Weak::new();
    println!("empty upgrade is_none={}", empty.upgrade().is_none());
}
