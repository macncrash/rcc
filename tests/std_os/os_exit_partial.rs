// process::exit from nested calls flushes a pending partial line on stdout; no destructors run
struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn inner(n: u32) -> u32 {
    let _d = D("inner");
    if n == 3 {
        print!("partial line before exit");
        std::process::exit(42);
    }
    inner(n + 1) + 1
}
fn main() {
    let _d = D("main");
    println!("start");
    {
        let _s = D("scope");
    }
    println!("result {}", inner(0));
}
