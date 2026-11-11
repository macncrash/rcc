// Fields are only read through derived Debug, which rustc ignores for dead-code analysis.
#![allow(dead_code)]

#[derive(Debug)]
struct Unit;

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Pair(i64, &'static str);

#[derive(Debug)]
enum Op {
    Nop,
    Load { dst: u8, addr: u32 },
    Add(u8, u8, u8),
    Call(String, Vec<Point>),
}

#[derive(Debug)]
struct Generic<T> {
    inner: T,
    list: Vec<T>,
}

fn main() {
    println!("{:?}", Unit);
    println!("{:?}", Point { x: 1, y: -2 });
    println!("{:?}", Pair(-9, "nine"));
    let ops = vec![
        Op::Nop,
        Op::Load { dst: 3, addr: 0x1000 },
        Op::Add(1, 2, 3),
        Op::Call("printf".to_string(), vec![Point { x: 0, y: 0 }]),
    ];
    for op in &ops {
        println!("{:?}", op);
    }
    println!("{:?}", ops);
    println!("{:?}", Generic { inner: 'c', list: vec!['a', 'b'] });
    println!("{:?}", Generic { inner: Some(1.5), list: vec![None, Some(-0.0)] });
    println!("{:?} {:?}", Some(3), None::<i32>);
    let ok: Result<u8, String> = Ok(1);
    let err: Result<u8, String> = Err("bad".into());
    println!("{:?} {:?}", ok, err);
    println!("{:?}", (1, "two", 3.0, '4', [5u8; 2]));
    println!("{:?}", (1,));
    println!("{:?}", ((), ((),)));
    println!("{:?}", "string with \"quotes\"");
    println!("{:?}", String::from("owned"));
    println!("{:?}", vec![vec![1, 2], vec![], vec![3]]);
    println!("{:?}", [Some(Unit), None]);
    println!("{:?}", Box::new(Point { x: 5, y: 6 }));
    println!("{:?}", &&Pair(0, ""));
    let s: &[u8] = b"hi\n";
    println!("{:?}", s);
    println!("{:?}", b"bytes");
    println!("{:?} {:?}", true, 'x');
    println!("{:?} {:?} {:?}", 1.0f64, 0.1f32, 1e100);
}
