enum Dir {
    N,
    E,
    S,
    W,
}

impl Dir {
    fn turn_right(&self) -> Dir {
        match self {
            Dir::N => Dir::E,
            Dir::E => Dir::S,
            Dir::S => Dir::W,
            Dir::W => Dir::N,
        }
    }
    fn delta(&self) -> (i32, i32) {
        match self {
            Dir::N => (0, 1),
            Dir::E => (1, 0),
            Dir::S => (0, -1),
            Dir::W => (-1, 0),
        }
    }
    fn letter(&self) -> char {
        match self {
            Dir::N => 'N',
            Dir::E => 'E',
            Dir::S => 'S',
            Dir::W => 'W',
        }
    }
    fn from_index(i: u32) -> Dir {
        match i % 4 {
            0 => Dir::N,
            1 => Dir::E,
            2 => Dir::S,
            _ => Dir::W,
        }
    }
}

enum Expr {
    Lit(i64),
    Add(i64, i64),
    Scale { by: i64, val: i64 },
}

impl Expr {
    fn eval(&self) -> i64 {
        match self {
            Self::Lit(v) => *v,
            Self::Add(a, b) => a + b,
            Self::Scale { by, val } => by * val,
        }
    }
    fn bump(&mut self) {
        match self {
            Self::Lit(v) => *v += 1,
            Self::Add(a, _) => *a += 10,
            Self::Scale { by, .. } => *by += 1,
        }
    }
}

fn main() {
    let mut d = Dir::N;
    let (mut x, mut y) = (0, 0);
    let moves = "RFFRFLFFLLF";
    let bytes = moves.as_bytes();
    for i in 0..bytes.len() {
        match bytes[i] {
            b'R' => d = d.turn_right(),
            b'L' => d = d.turn_right().turn_right().turn_right(),
            _ => {
                let (dx, dy) = d.delta();
                x += dx;
                y += dy;
            }
        }
        println!("{} {} {} {}", i, d.letter(), x, y);
    }
    for i in 0..6 {
        print!("{}", Dir::from_index(i).letter());
    }
    println!();
    let mut es = [Expr::Lit(3), Expr::Add(4, 5), Expr::Scale { by: 2, val: 21 }];
    for _ in 0..2 {
        for i in 0..es.len() {
            println!("eval {}", es[i].eval());
            es[i].bump();
        }
    }
}
