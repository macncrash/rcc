// Panic path: todo!("later") (expected exit code 101, message on stderr).
fn main() {
    println!("before");
    let v = vec![1];
    if v.len() == 1 {
        todo!("later");
    }
    println!("after");
}
