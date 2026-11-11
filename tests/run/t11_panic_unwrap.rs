// unwrap on None / expect on Err panic with exit 101
fn parse_all(items: &[&str]) -> Vec<i32> {
    let mut out = Vec::new();
    for s in items {
        let v: i32 = s.parse().expect("not a number");
        println!("parsed {}", v);
        out.push(v);
    }
    out
}

fn main() {
    let ok = parse_all(&["1", "22", "-3"]);
    println!("ok {:?}", ok);
    let first_even = ok.iter().find(|&&x| x % 2 == 0);
    println!("first even {}", first_even.unwrap());
    let none: Option<i32> = ok.iter().copied().find(|&x| x > 100);
    println!("none is_none {}", none.is_none());
    let bad = parse_all(&["7", "eight", "9"]);
    println!("unreachable {:?}", bad);
}
