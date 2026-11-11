// Mixing generics and dyn: generic fns accepting ?Sized dyn, generic structs
// holding Box<dyn>, dyn Trait<T> with generic parameter, assoc type in dyn.
trait Visitor<T> {
    fn visit(&mut self, t: &T);
    fn result(&self) -> String;
}

struct Summer(i64);
impl Visitor<i64> for Summer {
    fn visit(&mut self, t: &i64) {
        self.0 += *t;
    }
    fn result(&self) -> String {
        format!("sum={}", self.0)
    }
}
struct Lister(Vec<String>);
impl Visitor<i64> for Lister {
    fn visit(&mut self, t: &i64) {
        self.0.push(t.to_string());
    }
    fn result(&self) -> String {
        self.0.join("|")
    }
}
impl Visitor<&str> for Lister {
    fn visit(&mut self, t: &&str) {
        self.0.push(t.to_uppercase());
    }
    fn result(&self) -> String {
        self.0.join("+")
    }
}

fn walk<T>(items: &[T], v: &mut dyn Visitor<T>) {
    for i in items {
        v.visit(i);
    }
}

trait Describe {
    fn describe(&self) -> String;
}
impl Describe for i32 {
    fn describe(&self) -> String {
        format!("i32:{}", self)
    }
}
impl Describe for &str {
    fn describe(&self) -> String {
        format!("str:{}", self)
    }
}

fn show<T: Describe + ?Sized>(t: &T) -> String {
    t.describe()
}
impl Describe for dyn Visitor<i64> {
    fn describe(&self) -> String {
        format!("visitor({})", self.result())
    }
}

struct Holder<T> {
    items: Vec<Box<dyn Fn(&T) -> bool>>,
}
impl<T> Holder<T> {
    fn count(&self, t: &T) -> usize {
        self.items.iter().filter(|f| f(t)).count()
    }
}

trait Source {
    type Item;
    fn produce(&mut self) -> Option<Self::Item>;
}
struct Down(u8);
impl Source for Down {
    type Item = u8;
    fn produce(&mut self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}
fn drain(s: &mut dyn Source<Item = u8>) -> Vec<u8> {
    let mut v = Vec::new();
    while let Some(x) = s.produce() {
        v.push(x);
    }
    v
}

fn main() {
    let nums = [1i64, 2, 3, 40];
    let mut s = Summer(0);
    walk(&nums, &mut s);
    println!("{}", s.result());
    let mut l = Lister(Vec::new());
    walk(&nums, &mut l);
    println!("{}", Visitor::<i64>::result(&l));
    let mut l2 = Lister(Vec::new());
    walk(&["ab", "cd"], &mut l2);
    println!("{}", Visitor::<&str>::result(&l2));
    let vs: Vec<Box<dyn Visitor<i64>>> = vec![Box::new(Summer(5)), Box::new(Lister(vec![String::from("z")]))];
    for v in &vs {
        println!("{}", show(&**v));
    }
    println!("{} {}", show(&3), show(&"x"));
    let h: Holder<i32> = Holder { items: vec![Box::new(|x| *x > 0), Box::new(|x| x % 2 == 0), Box::new(|x| *x == 4)] };
    for x in [-2, 3, 4] {
        println!("{} matches {}", x, h.count(&x));
    }
    println!("{:?}", drain(&mut Down(4)));
}
