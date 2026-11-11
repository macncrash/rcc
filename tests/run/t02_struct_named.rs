struct Point {
    x: i32,
    y: i32,
}

struct Rect {
    origin: Point,
    w: u32,
    h: u32,
}

fn area(r: &Rect) -> u32 {
    r.w * r.h
}

fn shift(p: &mut Point, dx: i32, dy: i32) {
    p.x += dx;
    p.y += dy;
}

fn main() {
    let p = Point { x: 3, y: -4 };
    println!("{} {}", p.x, p.y);
    let mut q = Point { x: 10, ..p };
    println!("{} {}", q.x, q.y);
    shift(&mut q, 5, 6);
    println!("{} {}", q.x, q.y);
    let x = 7;
    let y = 8;
    let s = Point { x, y };
    println!("shorthand {} {}", s.x, s.y);
    let mut r = Rect { origin: Point { x: 1, y: 2 }, w: 30, h: 4 };
    println!("area {}", area(&r));
    r.w = 11;
    r.origin.x = 100;
    println!("area {} origin {} {}", area(&r), r.origin.x, r.origin.y);
    let Point { x: a, y: b } = q;
    println!("destructured {} {}", a, b);
    let rr = &mut r;
    rr.h += 1;
    rr.origin.y -= 50;
    println!("{} {} {} {}", r.origin.x, r.origin.y, r.w, r.h);
    let pts = [Point { x: 1, y: 1 }, Point { x: 2, y: 4 }, Point { x: 3, y: 9 }];
    let mut sum = 0;
    for i in 0..3 {
        sum += pts[i].x * pts[i].y;
        println!("pt {} -> {} {}", i, pts[i].x, pts[i].y);
    }
    println!("sum {}", sum);
}
