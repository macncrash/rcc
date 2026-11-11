// dyn Trait with supertraits: calling supertrait methods through the subtrait
// object, upcasting via helper methods, Debug as supertrait.
use std::fmt::Debug;

trait Named {
    fn name(&self) -> String;
}
trait Entity: Named + Debug {
    fn hp(&self) -> i32;
    fn summary(&self) -> String {
        format!("{} hp={}", self.name(), self.hp())
    }
    fn as_named(&self) -> &dyn Named;
}

#[derive(Debug)]
struct Orc {
    hp: i32,
}
#[derive(Debug)]
struct Elf {
    name: &'static str,
    hp: i32,
}

impl Named for Orc {
    fn name(&self) -> String {
        String::from("orc")
    }
}
impl Entity for Orc {
    fn hp(&self) -> i32 {
        self.hp
    }
    fn as_named(&self) -> &dyn Named {
        self
    }
}
impl Named for Elf {
    fn name(&self) -> String {
        format!("elf {}", self.name)
    }
}
impl Entity for Elf {
    fn hp(&self) -> i32 {
        self.hp
    }
    fn summary(&self) -> String {
        format!("[{}] {}", self.hp, self.name)
    }
    fn as_named(&self) -> &dyn Named {
        self
    }
}

fn print_name(n: &dyn Named) {
    println!("name: {}", n.name());
}

fn main() {
    let es: Vec<Box<dyn Entity>> = vec![Box::new(Orc { hp: 30 }), Box::new(Elf { name: "lin", hp: 12 }), Box::new(Orc { hp: -1 })];
    for e in &es {
        println!("{}", e.name());
        println!("{}", e.summary());
        println!("{:?}", e);
        print_name(e.as_named());
    }
    let alive = es.iter().filter(|e| e.hp() > 0).count();
    println!("alive {}", alive);
    let r: &dyn Entity = &*es[1];
    println!("{:?} {}", r, r.hp());
}
