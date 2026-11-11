// Box<dyn Trait> drops the concrete type; Box<T> recursive structures; Rc drop at last ref.
use std::rc::Rc;

trait Shape {
    fn name(&self) -> String;
}

struct Circle(u32);
struct Square(u32);

impl Shape for Circle {
    fn name(&self) -> String {
        format!("circle{}", self.0)
    }
}
impl Shape for Square {
    fn name(&self) -> String {
        format!("square{}", self.0)
    }
}
impl Drop for Circle {
    fn drop(&mut self) {
        println!("drop Circle {}", self.0);
    }
}
impl Drop for Square {
    fn drop(&mut self) {
        println!("drop Square {}", self.0);
    }
}

struct Node {
    val: u32,
    next: Option<Box<Node>>,
}
impl Drop for Node {
    fn drop(&mut self) {
        println!("drop Node {}", self.val);
    }
}

fn main() {
    let shapes: Vec<Box<dyn Shape>> = vec![Box::new(Circle(1)), Box::new(Square(2)), Box::new(Circle(3))];
    for s in &shapes {
        println!("{}", s.name());
    }
    println!("-- drop vec of Box<dyn>");
    drop(shapes);
    println!("-- reassign a Box<dyn>");
    let mut cur: Box<dyn Shape> = Box::new(Square(10));
    println!("cur {}", cur.name());
    cur = Box::new(Circle(11));
    println!("cur {}", cur.name());
    println!("-- linked list");
    let list = Node { val: 1, next: Some(Box::new(Node { val: 2, next: Some(Box::new(Node { val: 3, next: None })) })) };
    let mut p = &list;
    loop {
        println!("visit {}", p.val);
        match &p.next {
            Some(n) => p = n,
            None => break,
        }
    }
    drop(list);
    println!("-- Rc");
    let r1: Rc<dyn Shape> = Rc::new(Circle(20));
    let r2 = Rc::clone(&r1);
    println!("count {}", Rc::strong_count(&r1));
    drop(r1);
    println!("after dropping r1, count {}", Rc::strong_count(&r2));
    drop(r2);
    println!("after dropping r2");
    println!("end of main");
}
