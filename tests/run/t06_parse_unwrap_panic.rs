fn main() {
    let inputs = ["12", "34", "x5", "6"];
    let mut sum = 0;
    for s in inputs {
        println!("parsing {}", s);
        let n: i32 = s.parse().unwrap();
        sum += n;
        println!("sum {}", sum);
    }
    println!("done {}", sum);
}
