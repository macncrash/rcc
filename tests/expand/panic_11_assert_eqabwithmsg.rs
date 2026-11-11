// Panic path: assert_eq!("a", "b", "with {}", "msg") (expected exit code 101, message on stderr).
fn main() {
    println!("before");
    let v = vec![1];
    if v.len() == 1 {
        assert_eq!("a", "b", "with {}", "msg");
    }
    println!("after");
}
