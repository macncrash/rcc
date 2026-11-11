// derive(Debug) on structs, tuple structs, unit, enums, nested and generic types.
#![allow(dead_code)] // fields are only observed through Debug
#[derive(Debug)]
struct Unit;

#[derive(Debug)]
struct Tup(i32, &'static str, char);

#[derive(Debug)]
struct Named {
    id: u32,
    tag: Option<&'static str>,
    pt: (f64, f64),
}

#[derive(Debug)]
enum Expr {
    Lit(i64),
    Var { name: String, depth: u8 },
    Bin(Box<Expr>, char, Box<Expr>),
    Nil,
}

#[derive(Debug)]
struct Gen<T> {
    val: T,
    list: Vec<T>,
}

#[derive(Debug)]
enum Res<T, E> {
    Good(T),
    Bad(E),
}

fn main() {
    println!("{:?}", Unit);
    println!("{:?}", Tup(-1, "two", '3'));
    println!("{:?}", Named { id: 7, tag: Some("t"), pt: (1.5, -0.0) });
    println!("{:?}", Named { id: 0, tag: None, pt: (1e10, 2.0) });
    let e = Expr::Bin(
        Box::new(Expr::Lit(1)),
        '+',
        Box::new(Expr::Var { name: String::from("x\"y"), depth: 2 }),
    );
    println!("{:?}", e);
    println!("{:?}", Expr::Nil);
    println!("{:?}", Gen { val: 'a', list: vec!['b', '\n'] });
    println!("{:?}", Gen { val: (), list: vec![(), ()] });
    let r: Res<u8, String> = Res::Bad(String::from("oops"));
    println!("{:?}", r);
    let g: Res<Unit, ()> = Res::Good(Unit);
    println!("{:?}", g);
    println!("{:#?}", Named { id: 3, tag: Some("pretty"), pt: (0.1, 0.2) });
    println!("{:#?}", e);
}
