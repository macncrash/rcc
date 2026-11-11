// Forwarding impls: impl Trait for Box<dyn Trait> and &T so trait objects can be
// passed to generic code; generic fns instantiated with dyn-backed types;
// const/static trait objects.
trait Codegen {
    fn emit_op(&mut self, op: &str) -> usize;
    fn emit_prologue(&mut self) -> usize {
        self.emit_op("push fp") + self.emit_op("mov fp, sp")
    }
    fn emit_epilogue(&mut self) -> usize {
        self.emit_op("pop fp") + self.emit_op("ret")
    }
    fn arch(&self) -> &'static str;
}

struct X86 {
    out: Vec<String>,
}
struct Arm {
    out: Vec<String>,
}
impl Codegen for X86 {
    fn emit_op(&mut self, op: &str) -> usize {
        self.out.push(format!("  {}", op));
        1
    }
    fn arch(&self) -> &'static str {
        "x86"
    }
}
impl Codegen for Arm {
    fn emit_op(&mut self, op: &str) -> usize {
        self.out.push(format!("\t{}", op.replace("fp", "x29")));
        4
    }
    fn emit_prologue(&mut self) -> usize {
        self.emit_op("stp x29, x30, [sp, -16]!")
    }
    fn arch(&self) -> &'static str {
        "arm"
    }
}

impl<C: Codegen + ?Sized> Codegen for Box<C> {
    fn emit_op(&mut self, op: &str) -> usize {
        (**self).emit_op(op)
    }
    fn emit_prologue(&mut self) -> usize {
        (**self).emit_prologue()
    }
    fn emit_epilogue(&mut self) -> usize {
        (**self).emit_epilogue()
    }
    fn arch(&self) -> &'static str {
        (**self).arch()
    }
}

fn gen_function<C: Codegen>(cg: &mut C, body: &[&str]) -> usize {
    let mut size = cg.emit_prologue();
    for op in body {
        size += cg.emit_op(op);
    }
    size + cg.emit_epilogue()
}

trait Greeting {
    fn hello(&self) -> String;
}
struct En;
struct Fr;
impl Greeting for En {
    fn hello(&self) -> String {
        String::from("hello")
    }
}
impl Greeting for Fr {
    fn hello(&self) -> String {
        String::from("bonjour")
    }
}
static GREETERS: [&(dyn Greeting + Sync); 2] = [&En, &Fr];

fn main() {
    let mut x = X86 { out: Vec::new() };
    let n = gen_function(&mut x, &["mov rax, 1", "add rax, rdi"]);
    println!("x86 size {}", n);
    for l in &x.out {
        println!("{}", l);
    }
    let mut backends: Vec<Box<dyn Codegen>> = vec![Box::new(X86 { out: vec![] }), Box::new(Arm { out: vec![] })];
    for b in backends.iter_mut() {
        let n = gen_function(b, &["nop"]);
        println!("{} size {}", b.arch(), n);
    }
    let mut d: &mut dyn Codegen = &mut x;
    println!("{}", d.emit_op("int3"));
    d = backends[1].as_mut();
    println!("{} {}", d.arch(), d.emit_op("brk"));
    if let Some(last) = x.out.last() {
        println!("last {}", last);
    }
    for g in GREETERS.iter() {
        println!("{}", g.hello());
    }
}
