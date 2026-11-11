fn grade(score: u32) -> char {
    match score {
        90..=100 => 'A',
        80..=89 => 'B',
        70..=79 => 'C',
        0..=69 => 'F',
        _ => '?',
    }
}

fn kind(c: char) -> &'static str {
    match c {
        'a'..='z' => "lower",
        'A'..='Z' => "upper",
        '0'..='9' => "digit",
        ' ' | '\t' | '\n' => "space",
        _ => "other",
    }
}

fn sign(x: i64) -> i32 {
    match x {
        i64::MIN..=-1 => -1,
        0 => 0,
        1..=i64::MAX => 1,
    }
}

fn bucket(x: u8) -> u8 {
    match x {
        0..=9 => 0,
        10..=99 => 1,
        100..=u8::MAX => 2,
    }
}

const LOW: i32 = 10;
const HIGH: i32 = 20;

fn in_band(x: i32) -> bool {
    match x {
        LOW..=HIGH => true,
        _ => false,
    }
}

fn main() {
    let vals = [100, 95, 89, 80, 75, 3, 0, 101];
    for k in 0..vals.len() {
        let s = vals[k];
        println!("{} {}", s, grade(s));
    }
    let vals = ['q', 'Q', '5', ' ', '#', '\n'];
    for k in 0..vals.len() {
        let c = vals[k];
        println!("{} {}", c as u32, kind(c));
    }
    let vals = [i64::MIN, -5, 0, 7, i64::MAX];
    for k in 0..vals.len() {
        let x = vals[k];
        println!("{} {}", x, sign(x));
    }
    let vals = [0u8, 9, 10, 99, 100, 255];
    for k in 0..vals.len() {
        let x = vals[k];
        println!("{} {}", x, bucket(x));
    }
    let vals = [9, 10, 15, 20, 21];
    for k in 0..vals.len() {
        let x = vals[k];
        println!("{} {}", x, in_band(x));
    }
}
