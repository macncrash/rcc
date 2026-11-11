// Box<dyn Trait>: returned from factories, moved around, stored in Option,
// methods taking self: Box<Self>.
trait Animal {
    fn speak(&self) -> String;
    fn rename(&mut self, n: &str);
    fn into_name(self: Box<Self>) -> String;
}

struct Dog {
    name: String,
}
struct Cat {
    name: String,
    lives: u8,
}

impl Animal for Dog {
    fn speak(&self) -> String {
        format!("{}: woof", self.name)
    }
    fn rename(&mut self, n: &str) {
        self.name = n.to_string();
    }
    fn into_name(self: Box<Self>) -> String {
        self.name
    }
}
impl Animal for Cat {
    fn speak(&self) -> String {
        format!("{}: meow ({} lives)", self.name, self.lives)
    }
    fn rename(&mut self, n: &str) {
        self.name = format!("{} the cat", n);
        self.lives -= 1;
    }
    fn into_name(self: Box<Self>) -> String {
        format!("cat:{}", self.name)
    }
}

fn make(kind: u8, name: &str) -> Box<dyn Animal> {
    if kind == 0 {
        Box::new(Dog { name: name.to_string() })
    } else {
        Box::new(Cat { name: name.to_string(), lives: 9 })
    }
}

fn loudest(a: Box<dyn Animal>, b: Box<dyn Animal>) -> Box<dyn Animal> {
    if a.speak().len() >= b.speak().len() { a } else { b }
}

fn main() {
    let mut d = make(0, "rex");
    let mut c = make(1, "tom");
    println!("{}", d.speak());
    println!("{}", c.speak());
    d.rename("max");
    c.rename("felix");
    println!("{}", d.speak());
    println!("{}", c.speak());
    let w = loudest(d, c);
    println!("loudest {}", w.speak());
    println!("{}", w.into_name());
    let mut slot: Option<Box<dyn Animal>> = None;
    println!("{}", slot.is_none());
    slot = Some(make(0, "fido"));
    if let Some(a) = slot.as_mut() {
        a.rename("odie");
    }
    if let Some(a) = &slot {
        println!("{}", a.speak());
    }
    let taken = slot.take().unwrap();
    println!("{} {}", taken.into_name(), slot.is_none());
    let b: Box<dyn Animal> = Box::new(Cat { name: String::from("x"), lives: 3 });
    let r: &dyn Animal = &*b;
    println!("{}", r.speak());
}
