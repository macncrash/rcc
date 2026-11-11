// Macro hygiene for local variables, stringify!, concat!, line!-free builtins.
macro_rules! using_a {
    ($e:expr) => {{
        let a = 42;
        $e + a
    }};
}

macro_rules! declare_x {
    ($name:ident) => {
        let $name = 100;
        #[allow(unused_variables)]
        let x = -1; // hygienic: does not clash with caller's x
    };
}

macro_rules! swap_vals {
    ($a:ident, $b:ident) => {
        let tmp = $a;
        $a = $b;
        $b = tmp;
    };
}

macro_rules! show {
    ($e:expr) => {
        println!("{} = {:?}", stringify!($e), $e)
    };
}

macro_rules! names {
    ($($i:ident)*) => {
        [$(stringify!($i)),*]
    };
}

fn main() {
    let a = 1;
    // the macro's `a` is 42, the caller's `a` stays 1
    println!("{}", using_a!(a * 10));
    let x = 5;
    declare_x!(y);
    println!("x={} y={}", x, y);
    let mut p = 1;
    let mut q = 2;
    let tmp = 99;
    swap_vals!(p, q);
    println!("p={} q={} tmp={}", p, q, tmp);
    show!(1 + 2 * 3);
    show!(p * q);
    show!("text");
    show!([1, 2].len());
    println!("{:?}", names!(foo bar baz));
    println!("{}", concat!("a", 1, 'b', 2.5, true, -3));
    println!("{}", stringify!(struct S { x: u8 }));
    let s: &'static str = concat!("con", "cat");
    println!("{} {}", s, s.len());
}
