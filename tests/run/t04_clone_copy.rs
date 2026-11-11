// Clone vs Copy semantics: copies are independent, clones deep-copy, hand Clone.
#[derive(Debug, Clone, Copy, PartialEq)]
struct P {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone)]
struct Bag {
    items: Vec<P>,
    label: String,
}

struct Counted {
    v: u32,
    gen: u32,
}

impl Clone for Counted {
    fn clone(&self) -> Self {
        println!("cloning gen {}", self.gen);
        Counted { v: self.v, gen: self.gen + 1 }
    }
}

fn takes_copy(mut p: P) -> P {
    p.x += 100;
    p
}

fn dup<T: Clone>(t: &T) -> (T, T) {
    (t.clone(), t.clone())
}

fn main() {
    let a = P { x: 1, y: 2 };
    let mut b = a;
    b.x = 50;
    println!("{:?} {:?}", a, b);
    let c = takes_copy(a);
    println!("{:?} {:?}", a, c);
    let arr = [a; 3];
    let mut arr2 = arr;
    arr2[0].y = -1;
    println!("{:?} {:?}", arr[0], arr2[0]);

    let bag = Bag { items: vec![a, b], label: String::from("one") };
    let mut bag2 = bag.clone();
    bag2.items.push(c);
    bag2.label.push_str("-copy");
    bag2.items[0].x = 999;
    println!("{:?}", bag);
    println!("{:?}", bag2);

    let k = Counted { v: 7, gen: 0 };
    let k2 = k.clone();
    let k3 = k2.clone();
    println!("{} {} {} {}", k.v, k.gen, k2.gen, k3.gen);
    let (d1, d2) = dup(&k3);
    println!("{} {}", d1.gen, d2.gen);
    let (s1, mut s2) = dup(&String::from("hi"));
    s2.push('!');
    println!("{} {}", s1, s2);
    let t = (a, 5u8, 'c');
    let t2 = t;
    println!("{:?} {:?}", t, t2);
    let mut o = Some(a);
    let o2 = o;
    if let Some(p) = o.as_mut() {
        p.y = 77;
    }
    println!("{:?} {:?}", o, o2);
    let mut target = P { x: 0, y: 0 };
    target.clone_from(&c);
    println!("{:?}", target);
}
