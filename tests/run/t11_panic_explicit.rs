// explicit panic! exits with 101; stdout before the panic is kept
fn check(v: i32) -> i32 {
    if v > 3 {
        panic!("value too large: {}", v);
    }
    v * 10
}

fn main() {
    for i in 0..10 {
        println!("check({}) = {}", i, check(i));
    }
    println!("unreachable");
}
