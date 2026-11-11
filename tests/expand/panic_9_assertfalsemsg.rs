// Panic path: assert!(false, "msg {}", 9) (expected exit code 101, message on stderr).
fn main() {
    println!("before");
    let v = vec![1];
    if v.len() == 1 {
        assert!(false, "msg {}", 9);
    }
    println!("after");
}
