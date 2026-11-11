// Supertraits: methods of the supertrait usable via the subtrait bound.
trait Named {
    fn name(&self) -> String;
}

trait Greeter: Named {
    fn greet(&self) -> String {
        format!("Hello, {}!", self.name())
    }
}

trait Loud: Greeter {
    fn shout(&self) -> String {
        let g = self.greet();
        g.to_uppercase()
    }
}

struct Person {
    first: &'static str,
    last: &'static str,
}

impl Named for Person {
    fn name(&self) -> String {
        format!("{} {}", self.first, self.last)
    }
}
impl Greeter for Person {}
impl Loud for Person {}

struct Robot(u32);
impl Named for Robot {
    fn name(&self) -> String {
        format!("unit-{}", self.0)
    }
}
impl Greeter for Robot {
    fn greet(&self) -> String {
        format!("BEEP {}", self.name())
    }
}
impl Loud for Robot {}

fn intro<T: Loud>(t: &T) {
    println!("{}", t.name());
    println!("{}", t.greet());
    println!("{}", t.shout());
}

trait Base {
    fn value(&self) -> i32;
}
trait Doubled: Base {
    fn doubled(&self) -> i32 {
        self.value() * 2
    }
}
trait Both: Doubled + Clone {}

#[derive(Clone)]
struct N(i32);
impl Base for N {
    fn value(&self) -> i32 {
        self.0
    }
}
impl Doubled for N {}
impl Both for N {}

fn use_both<T: Both>(t: &T) -> i32 {
    let c = t.clone();
    c.value() + t.doubled()
}

fn main() {
    intro(&Person { first: "Ada", last: "Lovelace" });
    intro(&Robot(42));
    println!("{}", use_both(&N(5)));
    println!("{}", use_both(&N(-1)));
}
