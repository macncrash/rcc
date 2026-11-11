// Disambiguating methods with the same name from different traits/inherent impls.
trait Pilot {
    fn fly(&self) -> String;
    fn id() -> u32;
}
trait Wizard {
    fn fly(&self) -> String;
    fn id() -> u32;
}

struct Human {
    name: &'static str,
}

impl Human {
    fn fly(&self) -> String {
        format!("{} waves arms", self.name)
    }
    fn id() -> u32 {
        0
    }
}
impl Pilot for Human {
    fn fly(&self) -> String {
        format!("captain {} speaking", self.name)
    }
    fn id() -> u32 {
        1
    }
}
impl Wizard for Human {
    fn fly(&self) -> String {
        format!("{} rides a broom", self.name)
    }
    fn id() -> u32 {
        2
    }
}

trait Animal {
    fn baby_name() -> String;
}
struct Dog;
impl Dog {
    fn baby_name() -> String {
        String::from("Spot")
    }
}
impl Animal for Dog {
    fn baby_name() -> String {
        String::from("puppy")
    }
}

fn through_pilot<T: Pilot>(t: &T) -> String {
    t.fly()
}

mod shapes {
    pub trait Area {
        fn area(&self) -> u32;
    }
    pub struct Sq(pub u32);
    impl Area for Sq {
        fn area(&self) -> u32 {
            self.0 * self.0
        }
    }
}

fn main() {
    let h = Human { name: "Ann" };
    println!("{}", h.fly());
    println!("{}", Pilot::fly(&h));
    println!("{}", Wizard::fly(&h));
    println!("{}", <Human as Wizard>::fly(&h));
    println!("{}", Human::fly(&h));
    println!("{} {} {}", Human::id(), <Human as Pilot>::id(), <Human as Wizard>::id());
    println!("{}", Dog::baby_name());
    println!("{}", <Dog as Animal>::baby_name());
    println!("{}", through_pilot(&h));
    let s = shapes::Sq(7);
    println!("{}", shapes::Area::area(&s));
    println!("{}", <shapes::Sq as shapes::Area>::area(&s));
    {
        use shapes::Area;
        println!("{}", s.area());
    }
    let f: fn(&Human) -> String = <Human as Wizard>::fly;
    println!("{}", f(&Human { name: "Bo" }));
    println!("{}", i32::max(3, 9));
    println!("{}", <i32 as std::ops::Add>::add(3, 9));
    println!("{}", <u8 as Into<u32>>::into(7) + 1);
}
