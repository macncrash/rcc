// Fields are only read through derived Debug, which rustc ignores for dead-code analysis.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
enum Ty {
    Int { bits: u8, signed: bool },
    Ptr(Box<Ty>),
    Array(Box<Ty>, usize),
    Void,
}

#[derive(Debug)]
struct Field {
    name: String,
    ty: Ty,
}

#[derive(Debug)]
struct Struct {
    name: &'static str,
    fields: Vec<Field>,
    attrs: BTreeMap<String, Option<i64>>,
    empty: Vec<u8>,
    tags: BTreeSet<char>,
    unit: (),
    tuple: (i32, &'static str),
}

#[derive(Debug)]
struct Empty {}

#[derive(Debug)]
struct Wrapper(Vec<Empty>);

fn main() {
    let int = Ty::Int { bits: 32, signed: true };
    let s = Struct {
        name: "node",
        fields: vec![
            Field { name: "next".into(), ty: Ty::Ptr(Box::new(Ty::Void)) },
            Field { name: "data".into(), ty: Ty::Array(Box::new(int.clone()), 4) },
            Field { name: "n".into(), ty: int },
        ],
        attrs: [("align".to_string(), Some(8)), ("packed".to_string(), None)].into_iter().collect(),
        empty: vec![],
        tags: ['x', 'a'].into_iter().collect(),
        unit: (),
        tuple: (-1, "t"),
    };
    println!("{:#?}", s);
    println!("{:#?}", Empty {});
    println!("{:#?}", Wrapper(vec![Empty {}, Empty {}]));
    println!("{:#?}", (1, "a"));
    println!("{:#?}", Some(Some(3)));
    println!("{:#?}", vec![1, 2]);
    let r: Result<(), Ty> = Err(Ty::Void);
    println!("{:#?}", r);
    println!("{:#?}", [[1u8; 2]; 2]);
    println!("{:#?}", "str\twith\ttabs");
    let m: BTreeMap<(i32, i32), Vec<&str>> = [((0, 1), vec!["a"]), ((2, 3), vec![])].into_iter().collect();
    println!("{:#?}", m);
}
