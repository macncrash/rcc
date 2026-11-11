// #[derive] of all supported traits on all shapes; output and hashing must
// match rustc's derives exactly.
#![allow(dead_code)]
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

fn h<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
struct Unit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
struct Tuple(i32, u8);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
struct Named {
    id: u32,
    name: String,
    tags: Vec<&'static str>,
    r#type: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct Empty {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
enum Fieldless {
    A,
    #[default]
    B,
    C,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
enum Disc {
    X = 10,
    Y = 2,
    Z,
    W = 0,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Shape {
    Point,
    Circle(u32),
    Rect { w: u32, h: u32 },
    Many(Vec<Shape>, Box<Shape>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Gen<'a, T: Clone, const N: usize>
where
    T: std::fmt::Debug,
{
    items: [T; N],
    label: &'a str,
}

#[derive(Debug, Default)]
struct DefGen<T> {
    v: Vec<T>,
    t: T,
}

#[derive(Debug, Clone, PartialEq)]
enum Tree<T> {
    Leaf(T),
    Node(Box<Tree<T>>, Box<Tree<T>>),
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct F(f64);

#[derive(Debug)]
struct Deep {
    a: Vec<Option<(i32, String)>>,
    b: Shape,
    c: (),
    d: [u8; 3],
}

#[derive(Clone, Copy)]
union U {
    i: u32,
    f: f32,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Single {
    Only(u8),
}

fn main() {
    let n = Named { id: 1, name: "n".into(), tags: vec!["t1", "t2"], r#type: Some(true) };
    println!("{:?}", n);
    println!("{:#?}", n);
    println!("{:?} {:?} {:?}", Unit, Tuple(-1, 2), Empty {});
    println!("{:#?}", Tuple(3, 4));
    println!("{:?} {:?}", Fieldless::default(), Named::default());
    let shapes = vec![
        Shape::Point,
        Shape::Circle(3),
        Shape::Rect { w: 1, h: 2 },
        Shape::Many(vec![Shape::Point, Shape::Circle(1)], Box::new(Shape::Rect { w: 5, h: 6 })),
    ];
    println!("{:?}", shapes);
    println!("{:#?}", shapes[3]);
    let mut sorted = shapes.clone();
    sorted.sort();
    sorted.reverse();
    println!("{:?}", sorted);
    for a in &shapes {
        for b in &shapes {
            print!("{}{} ", (a == b) as u8, match a.cmp(b) { std::cmp::Ordering::Less => '<', std::cmp::Ordering::Equal => '=', _ => '>' });
        }
        println!();
    }
    let ds = [Disc::X, Disc::Y, Disc::Z, Disc::W];
    let set: BTreeSet<Disc> = ds.iter().copied().collect();
    println!("{:?} {:?}", set, Disc::Z as u8);
    println!("{:?}", ds.iter().map(|d| d.partial_cmp(&Disc::Y)).collect::<Vec<_>>());
    println!("{:?} {:?}", Fieldless::A < Fieldless::C, Fieldless::B.max(Fieldless::A));
    let g: Gen<'_, i32, 3> = Gen { items: [1, 2, 3], label: "g" };
    let g2 = g.clone();
    println!("{:?} {} {:?}", g, g == g2, g.cmp(&g2));
    println!("{:?}", DefGen::<u8>::default());
    let t = Tree::Node(Box::new(Tree::Leaf(1)), Box::new(Tree::Leaf(2)));
    println!("{:?} {}", t, t.clone() == t);
    println!("{:?} {:?}", F(1.5).partial_cmp(&F(f64::NAN)), F(1.0) < F(2.0));
    let d = Deep { a: vec![Some((1, "x".into())), None], b: Shape::Circle(9), c: (), d: [1, 2, 3] };
    println!("{:#?}", d);
    let u = U { i: 0x3f800000 };
    let u2 = u;
    println!("{}", unsafe { u2.f });
    // hashing must match rustc's derive exactly (DefaultHasher is deterministic)
    println!("{} {} {}", h(&Unit), h(&Tuple(1, 2)), h(&n));
    println!("{} {} {}", h(&Fieldless::A), h(&Fieldless::C), h(&Disc::Y));
    println!("{} {}", h(&shapes), h(&Single::Only(4)));
    println!("{}", h(&g));
    let c = Tuple(5, 6);
    let c2 = c;
    println!("{:?} {:?}", c, c2);
}
