// Trait objects with std::any::Any downcasting and type-tagged dyn objects.
use std::any::Any;

trait Component: Any {
    fn name(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct Health(i32);
struct Pos {
    x: f32,
    y: f32,
}

impl Component for Health {
    fn name(&self) -> &'static str {
        "health"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
impl Component for Pos {
    fn name(&self) -> &'static str {
        "pos"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn describe(v: &dyn Any) -> String {
    if let Some(i) = v.downcast_ref::<i32>() {
        format!("i32 {}", i)
    } else if let Some(s) = v.downcast_ref::<String>() {
        format!("string {:?}", s)
    } else if let Some(s) = v.downcast_ref::<&str>() {
        format!("str {}", s)
    } else {
        String::from("unknown")
    }
}

fn main() {
    let mut comps: Vec<Box<dyn Component>> = vec![Box::new(Health(10)), Box::new(Pos { x: 1.5, y: -2.0 }), Box::new(Health(3))];
    for c in comps.iter_mut() {
        if let Some(h) = c.as_any_mut().downcast_mut::<Health>() {
            h.0 *= 2;
        }
    }
    for c in &comps {
        let a = c.as_any();
        if let Some(h) = a.downcast_ref::<Health>() {
            println!("{} {}", c.name(), h.0);
        } else if let Some(p) = a.downcast_ref::<Pos>() {
            println!("{} {} {}", c.name(), p.x, p.y);
        }
        println!("is pos: {}", a.is::<Pos>());
    }
    println!("{}", describe(&5i32));
    println!("{}", describe(&String::from("hi")));
    println!("{}", describe(&"lit"));
    println!("{}", describe(&1.0f64));
    let b: Box<dyn Any> = Box::new(77u8);
    match b.downcast::<u8>() {
        Ok(v) => println!("u8 {}", v),
        Err(_) => println!("not u8"),
    }
}
