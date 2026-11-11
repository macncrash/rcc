// a panic still flushes pending stdout; exit code 101
fn main() {
    println!("line 1");
    print!("pending ");
    let v: Vec<i32> = Vec::new();
    if v.is_empty() {
        panic!("boom {}", 7);
    }
}
