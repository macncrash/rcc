struct P { x: i32, y: i32 }
fn pair() -> (i32, String) { (7, String::from("seven")) }
fn main() {
    let (mut a, mut b) = (1, 2);
    (a, b) = (b, a);
    println!("{} {}", a, b);
    let mut s = String::new();
    let n;
    (n, s) = pair();
    println!("{} {}", n, s);
    let mut arr = [0; 3];
    [arr[0], _, arr[2]] = [5, 6, 7];
    println!("{:?}", arr);
    let mut p = P { x: 0, y: 0 };
    P { x: p.y, y: p.x } = P { x: 10, y: 20 };
    println!("{} {}", p.x, p.y);
    let mut v = vec![1, 2, 3];
    (v[0], v[2]) = (v[2], v[0]);
    println!("{:?}", v);
    let (mut c, mut d, mut e);
    (c, (d, e)) = (1u8, (2u16, 3u32));
    (c, ..) = (9, 0, 0);
    _ = pair();
    println!("{} {} {}", c, d, e);
}
