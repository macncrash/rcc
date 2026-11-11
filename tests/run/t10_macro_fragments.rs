// `$s:stmt` re-emitted as `$s;` yields redundant semicolons for `let` statements.
#![allow(redundant_semicolons)]
// Every common fragment specifier: ident ty tt pat literal block stmt path vis item lifetime meta.
macro_rules! make_fn {
    ($name:ident, $t:ty, $val:expr) => {
        fn $name() -> $t {
            $val
        }
    };
}
make_fn!(get_seven, u8, 7);
make_fn!(get_pi, f64, 3.25);

macro_rules! is_match {
    ($e:expr, $p:pat) => {
        match $e {
            $p => true,
            _ => false,
        }
    };
}

macro_rules! lit_str {
    ($l:literal) => {
        concat!("<", $l, ">")
    };
}

macro_rules! run_block {
    ($b:block) => {{
        println!("before block");
        let r = $b;
        println!("after block");
        r
    }};
}

macro_rules! run_stmts {
    (result $e:ident; $($s:stmt;)*) => {{
        $($s;)*
        $e
    }};
}

macro_rules! call_path {
    ($p:path) => {
        $p(10)
    };
}

macro_rules! def_struct {
    ($v:vis struct $n:ident { $($fv:vis $f:ident : $ft:ty),* }) => {
        #[derive(Debug)]
        $v struct $n { $($fv $f: $ft),* }
    };
}
def_struct!(pub struct Point { pub x: i32, pub y: i32 });

macro_rules! with_item {
    ($i:item) => {
        $i
    };
}
with_item!(
    fn from_item() -> &'static str {
        "item fragment"
    }
);

macro_rules! longest {
    ($lt:lifetime) => {
        fn pick<$lt>(a: &$lt str, b: &$lt str) -> &$lt str {
            if a.len() >= b.len() { a } else { b }
        }
    };
}
longest!('a);

macro_rules! attr_fn {
    (#[$m:meta] $name:ident) => {
        #[$m]
        fn $name() -> u32 {
            99
        }
    };
}
attr_fn!(#[inline] attr_target);

macro_rules! count_tts {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count_tts!($($tail)*) };
}

mod util {
    pub fn tenfold(x: i32) -> i32 {
        x * 10
    }
}

fn main() {
    println!("{} {}", get_seven(), get_pi());
    println!("{} {}", is_match!(Some(3), Some(1..=5)), is_match!(Some(9), Some(1..=5)));
    println!("{}", is_match!((1, 'x'), (_, 'x' | 'y')));
    println!("{}", lit_str!("hi"));
    println!("{}", lit_str!(42));
    let r = run_block!({
        let a = 6;
        a * 7
    });
    println!("block result {}", r);
    let total = run_stmts! {
        result t;
        let mut t = 1;
        t += 2;
        t *= 5;
    };
    println!("stmts result {:?}", total);
    println!("path {}", call_path!(util::tenfold));
    println!("path {}", call_path!(i32::wrapping_neg));
    let p = Point { x: 1, y: 2 };
    println!("{:?} {} {}", p, p.x, p.y);
    println!("{}", from_item());
    println!("{}", pick("abc", "de"));
    println!("{}", attr_target());
    println!("tts {}", count_tts!(a b (c d) [e] {f g h} , ;));
}
