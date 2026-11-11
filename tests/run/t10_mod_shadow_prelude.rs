// Shadowing names that come from the prelude or std.
mod mine {
    #[derive(Debug)]
    pub enum Option<T> {
        Nothing,
        Just(T),
    }
    pub fn drop(x: i32) -> i32 {
        x - 1
    }
    pub struct String {
        pub len: usize,
    }
}

fn min(a: i32, b: i32) -> i32 {
    // deliberately "wrong" min to prove the local one is called
    a * 100 + b
}

type Result<T> = std::result::Result<T, &'static str>;

fn parse_pos(x: i32) -> Result<i32> {
    if x >= 0 { Ok(x) } else { Err("negative") }
}

fn uses_mine() {
    use mine::Option;
    use mine::Option::*;
    let a: Option<i32> = Just(5);
    let b: Option<i32> = Nothing;
    println!("{:?} {:?}", a, b);
    // real std Option is still reachable by path
    let c: std::option::Option<i32> = std::option::Option::Some(3);
    println!("{:?}", c);
    let s = mine::String { len: 4 };
    println!("my string len {}", s.len);
    println!("my drop {}", mine::drop(10));
}

fn main() {
    println!("min = {}", min(1, 2));
    println!("std min = {}", std::cmp::min(1, 2));
    println!("{:?} {:?}", parse_pos(4), parse_pos(-4));
    uses_mine();
    let drop = 41; // a local named like a prelude function
    println!("{}", drop + 1);
    let vec = vec![1, 2, 3];
    println!("{:?}", vec);
    let i32 = 7i32; // a local named like a primitive type
    let x: i32 = i32 + 1;
    println!("{}", x);
}
