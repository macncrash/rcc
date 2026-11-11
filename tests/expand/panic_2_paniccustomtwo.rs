// Panic path: panic!("custom {} {:?}", 1, "two") (expected exit code 101, message on stderr).
fn main() {
    println!("before");
    let v = vec![1];
    if v.len() == 1 {
        panic!("custom {} {:?}", 1, "two");
    }
    println!("after");
}
