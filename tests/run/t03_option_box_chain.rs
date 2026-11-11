// Singly linked list with Option<Box<Node>> (the idiomatic form).
struct Node {
    val: u32,
    next: Option<Box<Node>>,
}

struct Stack {
    head: Option<Box<Node>>,
    len: usize,
}

impl Stack {
    fn new() -> Stack {
        Stack { head: None, len: 0 }
    }
    fn push(&mut self, v: u32) {
        let old = self.head.take();
        self.head = Some(Box::new(Node { val: v, next: old }));
        self.len += 1;
    }
    fn pop(&mut self) -> Option<u32> {
        match self.head.take() {
            None => None,
            Some(n) => {
                self.head = n.next;
                self.len -= 1;
                Some(n.val)
            }
        }
    }
    fn peek(&self) -> Option<&u32> {
        self.head.as_ref().map(node_val)
    }
    fn peek_mut(&mut self) -> Option<&mut u32> {
        match &mut self.head {
            Some(n) => Some(&mut n.val),
            None => None,
        }
    }
    fn sum(&self) -> u32 {
        let mut s = 0;
        let mut cur = &self.head;
        while let Some(n) = cur {
            s += n.val;
            cur = &n.next;
        }
        s
    }
    fn reverse(&mut self) {
        let mut prev: Option<Box<Node>> = None;
        let mut cur = self.head.take();
        while let Some(mut n) = cur {
            cur = n.next.take();
            n.next = prev;
            prev = Some(n);
        }
        self.head = prev;
    }
    fn print(&self) {
        let mut cur = self.head.as_ref();
        while let Some(n) = cur {
            print!("{} ", n.val);
            cur = n.next.as_ref();
        }
        println!("(len {})", self.len);
    }
}

fn node_val(n: &Box<Node>) -> &u32 {
    &n.val
}

fn main() {
    let mut s = Stack::new();
    println!("{}", s.peek().is_none());
    for i in 1..=8 {
        s.push(i * 3);
    }
    s.print();
    println!("peek {} sum {}", s.peek().unwrap(), s.sum());
    if let Some(top) = s.peek_mut() {
        *top = 1000;
    }
    s.print();
    s.reverse();
    s.print();
    for _ in 0..3 {
        match s.pop() {
            Some(v) => println!("pop {}", v),
            None => println!("empty"),
        }
    }
    s.print();
    while let Some(v) = s.pop() {
        print!("{},", v);
    }
    println!();
    println!("{} {}", s.pop().is_none(), s.len);
}
