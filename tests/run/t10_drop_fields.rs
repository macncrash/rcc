// Drop order of struct fields (declaration order), tuples, arrays, enum payloads,
// nested structs, and a Drop impl on the outer struct running before its fields.
struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Plain {
    first: D,
    second: D,
    third: D,
}

struct Outer {
    name: &'static str,
    a: D,
    inner: Plain,
    z: D,
}
impl Drop for Outer {
    fn drop(&mut self) {
        println!("drop Outer {} (fields still alive: {} {})", self.name, self.a.0, self.z.0);
    }
}

enum E {
    Empty,
    One(D),
    Two { x: D, y: D },
}

fn main() {
    println!("-- struct fields drop in declaration order");
    {
        let _p = Plain { third: D("p.third"), first: D("p.first"), second: D("p.second") };
        println!("{} {} {}", _p.first.0, _p.second.0, _p.third.0);
    }
    println!("-- outer Drop runs before fields");
    {
        let _o = Outer {
            name: "o",
            a: D("o.a"),
            inner: Plain { first: D("o.inner.first"), second: D("o.inner.second"), third: D("o.inner.third") },
            z: D("o.z"),
        };
        println!("inner {} {} {}", _o.inner.first.0, _o.inner.second.0, _o.inner.third.0);
    }
    println!("-- tuple");
    {
        let _t = (D("t.0"), D("t.1"), (D("t.2.0"), D("t.2.1")));
    }
    println!("-- array");
    {
        let _arr = [D("arr0"), D("arr1"), D("arr2")];
    }
    println!("-- enum payloads");
    {
        let es = [E::Empty, E::One(D("one")), E::Two { x: D("two.x"), y: D("two.y") }];
        for e in &es {
            match e {
                E::Empty => println!("Empty"),
                E::One(d) => println!("One {}", d.0),
                E::Two { x, y } => println!("Two {} {}", x.0, y.0),
            }
        }
    }
    println!("-- struct literal temporaries are moved, not dropped");
    {
        let p = Plain { first: D("m.first"), second: D("m.second"), third: D("m.third") };
        println!("built {}", p.second.0);
    }
    println!("end of main");
}
