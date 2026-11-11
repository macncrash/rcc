// std::process::exit with a nonzero code; code after exit never runs, Drop does not run
struct Noisy;
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop must not run");
    }
}

fn finish(code: i32) -> ! {
    println!("exiting with {}", code);
    std::process::exit(code)
}

fn main() {
    let _n = Noisy;
    let mut total = 0;
    for i in 0..10 {
        total += i;
        println!("i {} total {}", i, total);
        if total > 20 {
            finish(3);
        }
    }
    println!("unreachable");
}
