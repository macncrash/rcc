// Panic path: assert_ne!(2, 2) (expected exit code 101, message on stderr).
fn main() {
    println!("before");
    let v = vec![1];
    if v.len() == 1 {
        assert_ne!(2, 2);
    }
    println!("after");
}
