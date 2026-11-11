use std::fmt::Write;

#[derive(Debug, Clone, PartialEq)]
struct P { x: i32, y: i32 }

macro_rules! square {
    ($e:expr) => { $e * $e };
}

fn main() {
    let a = 5;
    let p = P { x: 1, y: 2 };
    println!("hello {} {:?} {a}", square!(a + 1), p);
    let v = vec![1, 2, 3];
    let s = format!("{:>5}|{:<5}|{:^5}|{:05.1}", v[0], v[1], v[2], 3.14159);
    println!("{}", s);
    let mut out = String::new();
    write!(out, "{0} {0:?} {1:x}", "q", 255).unwrap();
    writeln!(out).unwrap();
    print!("{}", out);
    assert!(matches!(p, P { x: 1, .. }));
    assert_eq!(p.clone(), p, "clone {}", 1);
    eprintln!("err {}", line!());
}
