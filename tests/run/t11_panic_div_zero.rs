// integer division by zero panics (exit 101) even in release
fn divide(a: i32, b: i32) -> i32 {
    a / b
}

fn main() {
    let divisors = [5, 3, 1, 0, 2];
    for &d in divisors.iter() {
        println!("100 / {} = {}", d, divide(100, d));
    }
    println!("unreachable");
}
