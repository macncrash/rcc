use std::cell::RefCell;
use std::rc::Rc;

type Link = Option<Rc<RefCell<DNode>>>;

struct DNode {
    val: i32,
    next: Link,
    prev: Option<std::rc::Weak<RefCell<DNode>>>,
}

struct DList {
    head: Link,
    tail: Link,
    len: usize,
}

impl DList {
    fn new() -> Self { DList { head: None, tail: None, len: 0 } }
    fn push_back(&mut self, v: i32) {
        let n = Rc::new(RefCell::new(DNode { val: v, next: None, prev: None }));
        match self.tail.take() {
            Some(t) => {
                n.borrow_mut().prev = Some(Rc::downgrade(&t));
                t.borrow_mut().next = Some(n.clone());
                self.tail = Some(n);
            }
            None => {
                self.head = Some(n.clone());
                self.tail = Some(n);
            }
        }
        self.len += 1;
    }
    fn pop_front(&mut self) -> Option<i32> {
        self.head.take().map(|h| {
            match h.borrow_mut().next.take() {
                Some(nx) => {
                    nx.borrow_mut().prev = None;
                    self.head = Some(nx);
                }
                None => {
                    self.tail = None;
                }
            }
            self.len -= 1;
            let v = h.borrow().val;
            v
        })
    }
    fn to_vec(&self) -> Vec<i32> {
        let mut out = vec![];
        let mut cur = self.head.clone();
        while let Some(n) = cur {
            out.push(n.borrow().val);
            cur = n.borrow().next.clone();
        }
        out
    }
    fn to_vec_rev(&self) -> Vec<i32> {
        let mut out = vec![];
        let mut cur = self.tail.clone();
        while let Some(n) = cur {
            out.push(n.borrow().val);
            cur = n.borrow().prev.as_ref().and_then(|w| w.upgrade());
        }
        out
    }
    fn map_in_place(&self, f: impl Fn(i32) -> i32) {
        let mut cur = self.head.clone();
        while let Some(n) = cur {
            let v = n.borrow().val;
            n.borrow_mut().val = f(v);
            cur = n.borrow().next.clone();
        }
    }
}

fn main() {
    let mut l = DList::new();
    for i in 1..=6 {
        l.push_back(i * i);
    }
    println!("len={} fwd={:?}", l.len, l.to_vec());
    println!("rev={:?}", l.to_vec_rev());
    l.map_in_place(|x| x - 1);
    println!("mapped={:?}", l.to_vec());
    println!("pop={:?} pop={:?}", l.pop_front(), l.pop_front());
    println!("len={} fwd={:?} rev={:?}", l.len, l.to_vec(), l.to_vec_rev());
    if let Some(h) = &l.head {
        println!("head strong={}", Rc::strong_count(h));
    }
    if let Some(t) = &l.tail {
        println!("tail strong={} weak={}", Rc::strong_count(t), Rc::weak_count(t));
    }
    while let Some(v) = l.pop_front() {
        print!("{} ", v);
    }
    println!("| empty={} tail_none={}", l.head.is_none(), l.tail.is_none());

    let shared = Rc::new(RefCell::new(Vec::<String>::new()));
    let writers: Vec<Rc<RefCell<Vec<String>>>> = (0..3).map(|_| shared.clone()).collect();
    for (i, w) in writers.iter().enumerate() {
        w.borrow_mut().push(format!("writer{}", i));
    }
    println!("{:?} strong={}", shared.borrow(), Rc::strong_count(&shared));
}
