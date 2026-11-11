fn main() {
    let v = vec![10, 20, 30];
    let mut total = 0;
    for i in 0..5 {
        println!("reading index {}", i);
        total += v[i];
        println!("total {}", total);
    }
    println!("unreachable {}", total);
}
