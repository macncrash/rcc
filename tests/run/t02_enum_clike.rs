enum Color {
    Red,
    Green,
    Blue,
}

enum Code {
    A = 1,
    B = 5,
    C,
    D = 100,
}

#[repr(u8)]
enum Small {
    X = 250,
    Y,
}

fn name(c: &Color) -> &'static str {
    match c {
        Color::Red => "red",
        Color::Green => "green",
        Color::Blue => "blue",
    }
}

fn next(c: Color) -> Color {
    match c {
        Color::Red => Color::Green,
        Color::Green => Color::Blue,
        Color::Blue => Color::Red,
    }
}

fn main() {
    let mut c = Color::Red;
    for i in 0..5 {
        println!("{} {} {}", i, name(&c), i);
        c = next(c);
    }
    println!("{} {} {} {}", Code::A as i32, Code::B as i32, Code::C as i32, Code::D as i32);
    println!("{} {}", Small::X as u8, Small::Y as u8);
    println!("{}", Color::Blue as u8);
    println!("{}", std::mem::size_of::<Small>());
    use Color::*;
    let mut total = 0;
    for k in 0..4 {
        let x = match k { 0 => Blue, 1 => Red, 2 => Green, _ => Blue };
        total = total * 3 + x as i32;
    }
    let last = next(next(c));
    println!("last {} {}", name(&last), last as i32);
    println!("total {}", total);
}
