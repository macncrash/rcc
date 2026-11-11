// &mut dyn Trait: state mutation through trait objects, swapping implementations
// at runtime, passing &mut dyn down recursive calls.
trait Emitter {
    fn emit(&mut self, s: &str);
    fn indent(&mut self) {}
    fn dedent(&mut self) {}
    fn finish(&self) -> String;
}

struct Flat {
    out: String,
}
impl Emitter for Flat {
    fn emit(&mut self, s: &str) {
        if !self.out.is_empty() {
            self.out.push(' ');
        }
        self.out.push_str(s);
    }
    fn finish(&self) -> String {
        self.out.clone()
    }
}

struct Pretty {
    lines: Vec<String>,
    depth: usize,
}
impl Emitter for Pretty {
    fn emit(&mut self, s: &str) {
        let mut l = String::new();
        for _ in 0..self.depth {
            l.push_str("  ");
        }
        l.push_str(s);
        self.lines.push(l);
    }
    fn indent(&mut self) {
        self.depth += 1;
    }
    fn dedent(&mut self) {
        self.depth -= 1;
    }
    fn finish(&self) -> String {
        self.lines.join("\n")
    }
}

enum Node {
    Leaf(&'static str),
    List(&'static str, Vec<Node>),
}

fn render(n: &Node, e: &mut dyn Emitter) {
    match n {
        Node::Leaf(s) => e.emit(s),
        Node::List(name, kids) => {
            e.emit(&format!("{} {{", name));
            e.indent();
            for k in kids {
                render(k, e);
            }
            e.dedent();
            e.emit("}");
        }
    }
}

fn main() {
    let tree = Node::List(
        "fn",
        vec![Node::Leaf("a"), Node::List("if", vec![Node::Leaf("b"), Node::Leaf("c")]), Node::Leaf("d")],
    );
    let mut flat = Flat { out: String::new() };
    let mut pretty = Pretty { lines: Vec::new(), depth: 0 };
    render(&tree, &mut flat);
    render(&tree, &mut pretty);
    println!("{}", flat.finish());
    println!("{}", pretty.finish());
    let mut emitters: [&mut dyn Emitter; 2] = [&mut flat, &mut pretty];
    for e in emitters.iter_mut() {
        e.emit("extra");
    }
    for (i, e) in emitters.iter().enumerate() {
        let f = e.finish();
        println!("{} last line: {}", i, f.lines().last().unwrap());
    }
    let mut which: Box<dyn Emitter> = Box::new(Flat { out: String::from("start") });
    which.emit("x");
    println!("{}", which.finish());
    which = Box::new(Pretty { lines: vec![], depth: 2 });
    which.emit("y");
    println!("{}", which.finish());
}
