// Panic path: assert!(1 + 1 == 3) (expected exit code 101, message on stderr).
fn main() {
    println!("before");
    let v = vec![1];
    if v.len() == 1 {
        assert!(1 + 1 == 3);
    }
    println!("after");
}
