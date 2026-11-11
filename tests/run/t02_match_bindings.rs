enum Msg {
    Move { x: i32, y: i32 },
    Write(&'static str),
    Color(u8, u8, u8),
    Quit,
}

fn handle(m: &Msg) -> i32 {
    match m {
        Msg::Move { x: 0, y } => {
            println!("vertical move {}", y);
            *y
        }
        Msg::Move { x, y: yy } => {
            println!("move {} {}", x, yy);
            x + yy
        }
        Msg::Write(t) => {
            println!("write {} len {}", t, t.len());
            t.len() as i32
        }
        Msg::Color(r, _, b) if *r > 200 => -(*b as i32),
        Msg::Color(r, g, b) => {
            println!("g {}", g);
            println!("color r {} b {}", r, b);
            *r as i32 + *b as i32
        }
        Msg::Quit => {
            println!("quit");
            -1
        }
    }
}

fn main() {
    let msgs = [
        Msg::Move { x: 0, y: 9 },
        Msg::Move { x: 3, y: -2 },
        Msg::Write("hello"),
        Msg::Color(10, 20, 30),
        Msg::Quit,
    ];
    let mut total = 0;
    for i in 0..msgs.len() {
        total += handle(&msgs[i]);
    }
    println!("total {}", total);
    let n = 42;
    let desc = match n {
        x if x < 0 => x * 2,
        x => x + 1,
    };
    println!("desc {}", desc);
    let pair = (5, -5);
    match pair {
        (a, b) if a + b == 0 => println!("opposites {} {}", a, b),
        (a, _) => println!("other {}", a),
    }
    let mut v = 3;
    match v {
        ref mut r => *r *= 7,
    }
    println!("v {}", v);
    let ref rr = v;
    println!("rr {}", *rr + 1);
}
