// slice index out of bounds panics with exit 101
fn get(v: &[u32], i: usize) -> u32 {
    v[i]
}

fn main() {
    let v = vec![10u32, 20, 30];
    let mut sum = 0;
    for i in 0..10 {
        let x = get(&v, i);
        sum += x;
        println!("v[{}] = {} sum {}", i, x, sum);
    }
    println!("unreachable");
}
