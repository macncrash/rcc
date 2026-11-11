use std::mem;

#[derive(Debug)]
enum Shape {
    Circle(f64),
    Rect { w: f64, h: f64 },
    Empty,
}

fn area(s: &Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.0 * r * r,
        Shape::Rect { w, h } => w * h,
        Shape::Empty => 0.0,
    }
}

#[derive(Debug, Default)]
struct Buf {
    data: Vec<u8>,
    name: String,
}

fn main() {
    let mut a = 1;
    let mut b = 2;
    mem::swap(&mut a, &mut b);
    println!("swap a={} b={}", a, b);
    let mut v1 = vec![1, 2, 3];
    let mut v2 = vec![9];
    mem::swap(&mut v1, &mut v2);
    println!("v1={:?} v2={:?}", v1, v2);
    let mut arr = [10, 20, 30, 40];
    arr.swap(0, 3);
    let (l, r) = arr.split_at_mut(2);
    mem::swap(&mut l[0], &mut r[0]);
    println!("arr={:?}", arr);

    let mut s = String::from("old");
    let prev = mem::replace(&mut s, String::from("new"));
    println!("replace prev={} s={}", prev, s);
    let mut buf = Buf { data: vec![1, 2, 3], name: "b1".into() };
    let taken = mem::take(&mut buf.data);
    println!("taken={:?} left={:?} name={}", taken, buf, buf.name);
    let whole = mem::take(&mut buf);
    println!("whole={:?} left={:?}", whole, buf);
    let mut opt = Some(5);
    let t = opt.take();
    println!("take={:?} opt={:?}", t, opt);
    let mut opt2 = Some(1);
    let old = opt2.replace(2);
    println!("replace old={:?} new={:?}", old, opt2);
    let mut n = 77u32;
    println!("take int={} left={}", mem::take(&mut n), n);

    let shapes = [Shape::Circle(1.0), Shape::Circle(2.5), Shape::Rect { w: 1.0, h: 2.0 }, Shape::Empty];
    for i in 0..shapes.len() {
        for j in 0..shapes.len() {
            let same = mem::discriminant(&shapes[i]) == mem::discriminant(&shapes[j]);
            print!("{}", if same { 'Y' } else { '.' });
        }
        println!();
    }
    let d1 = mem::discriminant(&Some(1));
    let d2 = mem::discriminant(&Some(999));
    let d3 = mem::discriminant(&None::<i32>);
    println!("some==some {} some==none {}", d1 == d2, d1 == d3);
    println!("{:?}", shapes);
    println!("areas {:?}", shapes.iter().map(area).collect::<Vec<_>>());
    println!("size u8={} u32={} u64={} u128={} (u8,u32)={}", mem::size_of::<u8>(), mem::size_of::<u32>(), mem::size_of::<u64>(), mem::size_of::<u128>(), mem::size_of::<[u32; 4]>());
    let x = vec![String::from("dropme")];
    mem::drop(x);
    let kept = String::from("forgotten");
    mem::forget(kept);
    println!("done");
}
