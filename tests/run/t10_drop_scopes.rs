// Drop order: reverse declaration order, nested blocks, `let _ =` vs `let _x =`.
struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn make(name: &'static str) -> D {
    println!("make {}", name);
    D(name)
}

fn inner_fn() {
    let _a = make("fn-a");
    let _b = make("fn-b");
    println!("leaving inner_fn");
}

fn main() {
    let _x = make("x");
    let _y = make("y");
    {
        let _inner1 = make("inner1");
        let _inner2 = make("inner2");
        println!("end of block");
    }
    println!("after block");
    let _ = make("wildcard"); // dropped immediately
    println!("after wildcard");
    let _named = make("named"); // lives to end of main
    inner_fn();
    let v = {
        let _tmp = make("block-tmp");
        42
    };
    println!("block value {}", v);
    let moved_out = {
        let keep = make("kept");
        let _gone = make("gone");
        keep
    };
    println!("got {}", moved_out.0);
    for i in 0..3 {
        let _it = D(["iter0", "iter1", "iter2"][i]);
        println!("loop body {}", i);
    }
    let mut n = 0;
    while n < 2 {
        let _w = make("while-body");
        n += 1;
    }
    println!("end of main");
}
