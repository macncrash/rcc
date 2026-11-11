// Recursive macros and tt-munchers: counting, reversing, tiny DSLs.
macro_rules! count {
    () => { 0usize };
    ($x:tt $($xs:tt)*) => { 1usize + count!($($xs)*) };
}

macro_rules! reverse_list {
    ([] -> [$($out:expr),*]) => { vec![$($out),*] };
    ([$first:expr $(, $rest:expr)*] -> [$($out:expr),*]) => {
        reverse_list!([$($rest),*] -> [$first $(, $out)*])
    };
}

// tt-muncher: evaluate a tiny RPN-like token stream left to right
macro_rules! calc {
    (@acc $acc:expr ;) => { $acc };
    (@acc $acc:expr ; plus $n:literal $($rest:tt)*) => { calc!(@acc ($acc + $n) ; $($rest)*) };
    (@acc $acc:expr ; times $n:literal $($rest:tt)*) => { calc!(@acc ($acc * $n) ; $($rest)*) };
    (@acc $acc:expr ; minus $n:literal $($rest:tt)*) => { calc!(@acc ($acc - $n) ; $($rest)*) };
    ($start:literal $($rest:tt)*) => { calc!(@acc $start ; $($rest)*) };
}

// build nested tuples recursively
macro_rules! nest {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => { ($x, nest!($($rest),+)) };
}

// key/value parser via munching with different value kinds
macro_rules! kv {
    (@out [$($done:expr),*]) => { vec![$($done),*] };
    (@out [$($done:expr),*] $k:ident = $v:literal, $($rest:tt)*) => {
        kv!(@out [$($done,)* format!("{}:{}", stringify!($k), $v)] $($rest)*)
    };
    (@out [$($done:expr),*] $k:ident, $($rest:tt)*) => {
        kv!(@out [$($done,)* format!("{}:flag", stringify!($k))] $($rest)*)
    };
    ($($t:tt)*) => { kv!(@out [] $($t)*) };
}

macro_rules! fib {
    (0) => { 0u64 };
    (1) => { 1u64 };
    ($n:expr) => {{
        let mut a = 0u64;
        let mut b = 1u64;
        for _ in 0..$n {
            let t = a + b;
            a = b;
            b = t;
        }
        a
    }};
}

fn main() {
    println!("{}", count!());
    println!("{}", count!(a));
    println!("{}", count!(a b c d e f g));
    println!("{:?}", reverse_list!([1, 2, 3, 4, 5] -> []));
    println!("{}", calc!(2 plus 3 times 4 minus 1));
    println!("{}", calc!(10));
    let t = nest!(1, 2, 3, 4);
    println!("{:?}", t);
    println!("{}", ((t.1).1).1);
    println!("{:?}", kv!(a = 1, verbose, name = "x", fast,));
    println!("{} {} {}", fib!(0), fib!(1), fib!(50));
}
