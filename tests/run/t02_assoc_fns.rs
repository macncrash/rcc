struct Vec2 {
    x: i64,
    y: i64,
}

impl Vec2 {
    const ZERO: Vec2 = Vec2 { x: 0, y: 0 };

    fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }
    fn unit_x() -> Self {
        Self::new(1, 0)
    }
    fn add(&self, o: &Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
    fn scale(self, k: i64) -> Self {
        Self { x: self.x * k, y: self.y * k }
    }
    fn dot(a: &Vec2, b: &Vec2) -> i64 {
        a.x * b.x + a.y * b.y
    }
    fn len2(&self) -> i64 {
        Self::dot(self, self)
    }
}

struct Matrix {
    m: [[i64; 2]; 2],
}

impl Matrix {
    fn identity() -> Matrix {
        Matrix { m: [[1, 0], [0, 1]] }
    }
    fn mul(&self, o: &Matrix) -> Matrix {
        let mut r = [[0i64; 2]; 2];
        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    r[i][j] += self.m[i][k] * o.m[k][j];
                }
            }
        }
        Matrix { m: r }
    }
    fn apply(&self, v: &Vec2) -> Vec2 {
        Vec2::new(self.m[0][0] * v.x + self.m[0][1] * v.y, self.m[1][0] * v.x + self.m[1][1] * v.y)
    }
}

fn main() {
    let z = Vec2::ZERO;
    println!("{} {}", z.x, z.y);
    let a = Vec2::unit_x().scale(3);
    let b = Vec2::new(-2, 5);
    let c = a.add(&b);
    println!("{} {} dot {} len2 {}", c.x, c.y, Vec2::dot(&a, &b), c.len2());
    let fib = Matrix { m: [[1, 1], [1, 0]] };
    let mut acc = Matrix::identity();
    for i in 1..=20 {
        acc = acc.mul(&fib);
        if i % 4 == 0 {
            println!("fib^{} = {} {} {} {}", i, acc.m[0][0], acc.m[0][1], acc.m[1][0], acc.m[1][1]);
        }
    }
    let rot = Matrix { m: [[0, -1], [1, 0]] };
    let mut v = Vec2::new(3, 1);
    for _ in 0..4 {
        v = rot.apply(&v);
        println!("rot {} {}", v.x, v.y);
    }
    let f = Vec2::len2;
    println!("fn path {}", f(&Vec2::new(6, 8)));
}
