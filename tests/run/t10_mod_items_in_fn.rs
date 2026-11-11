// Items declared inside function bodies and blocks.
fn compute(n: u32) -> u32 {
    const BASE: u32 = 10;
    static SCALE: u32 = 3;
    fn helper(x: u32) -> u32 {
        x * SCALE_OUTER
    }
    const SCALE_OUTER: u32 = 2;
    helper(n) + BASE * SCALE
}

fn shapes() {
    #[derive(Debug, Clone, Copy)]
    struct Pt {
        x: i32,
        y: i32,
    }
    impl Pt {
        fn manhattan(self) -> i32 {
            self.x.abs() + self.y.abs()
        }
    }
    enum Dir {
        N,
        S,
    }
    trait Flip {
        fn flip(&self) -> Self;
    }
    impl Flip for Pt {
        fn flip(&self) -> Pt {
            Pt { x: -self.y, y: self.x }
        }
    }
    let p = Pt { x: 3, y: -4 };
    println!("{:?} manhattan={}", p, p.manhattan());
    println!("{:?}", p.flip().flip());
    for d in [Dir::N, Dir::S] {
        let s = match d {
            Dir::N => "north",
            Dir::S => "south",
        };
        println!("{}", s);
    }
}

fn nested_blocks() -> i32 {
    let a = {
        fn f() -> i32 { 1 }
        f()
    };
    let b = {
        fn f() -> i32 { 20 }
        f()
    };
    let c = {
        mod m {
            pub fn g() -> i32 { 300 }
        }
        m::g()
    };
    a + b + c
}

fn main() {
    println!("compute(5) = {}", compute(5));
    println!("compute(0) = {}", compute(0));
    shapes();
    println!("nested = {}", nested_blocks());
    // item used before its declaration in the same block
    println!("late = {}", late());
    fn late() -> &'static str {
        "declared after use"
    }
    type Pair = (u8, u8);
    let p: Pair = (1, 2);
    println!("pair {} {}", p.0, p.1);
}
