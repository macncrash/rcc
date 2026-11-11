enum Shape {
    Circle(f64),
    Rect { w: f64, h: f64 },
    Tri(f64, f64, f64),
    Empty,
}

fn area(s: &Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.0 * r * r,
        Shape::Rect { w, h } => w * h,
        Shape::Tri(a, b, _) => a * b / 2.0,
        Shape::Empty => 0.0,
    }
}

fn kind(s: &Shape) -> &'static str {
    match *s {
        Shape::Circle(_) => "circle",
        Shape::Rect { .. } => "rect",
        Shape::Tri(..) => "tri",
        Shape::Empty => "empty",
    }
}

fn perimeter(s: &Shape) -> f64 {
    match s {
        Shape::Circle(r) => 6.0 * r,
        Shape::Rect { w, h } => 2.0 * (w + h),
        Shape::Tri(a, b, c) => a + b + c,
        Shape::Empty => 0.0,
    }
}

fn grow(s: &mut Shape) {
    match s {
        Shape::Circle(r) => *r += 1.0,
        Shape::Rect { w, .. } => *w *= 2.0,
        Shape::Tri(a, b, c) => {
            *a += 1.0;
            *b += 1.0;
            *c += 1.0;
        }
        Shape::Empty => *s = Shape::Circle(0.5),
    }
}

fn main() {
    let mut shapes = [
        Shape::Circle(2.0),
        Shape::Rect { w: 3.0, h: 4.5 },
        Shape::Tri(3.0, 4.0, 5.0),
        Shape::Empty,
    ];
    for round in 0..3 {
        for i in 0..4 {
            let s = &shapes[i];
            println!("{} {} {} {}", round, kind(s), area(s), perimeter(s));
        }
        for i in 0..4 {
            grow(&mut shapes[i]);
        }
    }
}
